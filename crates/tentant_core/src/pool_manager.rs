use crate::types::{TenantConfig, TenantId};
use dashmap::DashMap;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Debug, Error)]
pub enum PoolError {
    #[error("Failed to create pool: {0}")]
    CreationFailed(String),
    #[error("Failed to acquire connection: {0}")]
    AcquireFailed(String),
    #[error("Invalid connection string: {0}")]
    InvalidConnectionString(String),
    #[error("Tenant not found: {0}")]
    TenantNotFound(String),
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
}

/// Estadísticas de un pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub size: u32,
    pub idle: u32,
}

// Nota: sqlx no expone PoolState directamente en la API pública
// Usaremos el método size() que devuelve u32
impl PoolStats {
    pub fn from_pool(pool: &PgPool) -> Self {
        let size = pool.size();
        Self {
            size,
            idle: 0, // sqlx no expone idle connections en la API pública
        }
    }
}

/// Manager de pools multi-tenant usando sqlx
pub struct TenantPoolManager {
    /// Map de tenant_id -> PgPool
    pools: Arc<DashMap<TenantId, PgPool>>,
    /// Configuración por defecto
    default_max_connections: u32,
    default_min_connections: u32,
    acquire_timeout: Duration,
    idle_timeout: Duration,
}

impl TenantPoolManager {
    pub fn new(
        default_max_connections: u32,
        default_min_connections: u32,
        acquire_timeout_secs: u64,
        idle_timeout_secs: u64,
    ) -> Self {
        Self {
            pools: Arc::new(DashMap::new()),
            default_max_connections,
            default_min_connections,
            acquire_timeout: Duration::from_secs(acquire_timeout_secs),
            idle_timeout: Duration::from_secs(idle_timeout_secs),
        }
    }

    /// Crea una instancia con valores por defecto
    pub fn with_defaults() -> Self {
        Self::new(
            10,  // max connections
            2,   // min connections
            30,  // acquire timeout
            600, // idle timeout (10 min)
        )
    }

    /// Obtiene un pool existente o lo crea lazy
    pub async fn get_pool(&self, config: &TenantConfig) -> Result<PgPool, PoolError> {
        // Fast path: pool ya existe
        if let Some(pool) = self.pools.get(&config.id) {
            debug!(tenant_id = %config.id, "Pool cache hit");
            return Ok(pool.clone());
        }

        // Slow path: crear pool
        debug!(tenant_id = %config.id, "Pool cache miss, creating new pool");
        let pool = self.create_pool(config).await?;

        // Insert con check de race condition
        self.pools
            .entry(config.id.clone())
            .or_insert_with(|| pool.clone());

        info!(
            tenant_id = %config.id,
            max_conn = config.max_connections,
            "Created new pool for tenant"
        );

        Ok(pool)
    }

    /// Crea un nuevo pool de sqlx
    async fn create_pool(&self, config: &TenantConfig) -> Result<PgPool, PoolError> {
        let max_connections = if config.max_connections > 0 {
            config.max_connections
        } else {
            self.default_max_connections
        };

        // Parse connection string
        let connect_opts = PgConnectOptions::from_str(&config.connection_string)
            .map_err(|e| PoolError::InvalidConnectionString(e.to_string()))?
            .application_name(&format!("tenant-{}", config.id))
            .log_statements(tracing::log::LevelFilter::Debug)
            .log_slow_statements(tracing::log::LevelFilter::Warn, Duration::from_millis(500));

        // Crear pool con configuración
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(self.default_min_connections)
            .acquire_timeout(self.acquire_timeout)
            .idle_timeout(Some(self.idle_timeout))
            .max_lifetime(Some(Duration::from_secs(3600))) // 1 hora max lifetime
            .test_before_acquire(true) // Verifica conexión antes de entregar
            .connect_with(connect_opts)
            .await
            .map_err(|e| PoolError::CreationFailed(e.to_string()))?;

        Ok(pool)
    }

    /// Pre-calienta un pool (útil en eventos TenantCreated)
    pub async fn warm_pool(&self, config: &TenantConfig) -> Result<(), PoolError> {
        let pool = self.get_pool(config).await?;

        // Adquiere y libera una conexión para verificar conectividad
        let conn = pool
            .acquire()
            .await
            .map_err(|e| PoolError::AcquireFailed(e.to_string()))?;

        drop(conn); // Liberar inmediatamente

        info!(tenant_id = %config.id, "Pool warmed successfully");
        Ok(())
    }

    /// Remueve un pool (útil en eventos TenantDeactivated)
    pub async fn remove_pool(&self, tenant_id: &TenantId) {
        if let Some((_, pool)) = self.pools.remove(tenant_id) {
            pool.close().await;
            info!(tenant_id = %tenant_id, "Pool removed and closed");
        }
    }

    /// Obtiene estadísticas de un pool
    pub fn get_stats(&self, tenant_id: &TenantId) -> Option<PoolStats> {
        self.pools
            .get(tenant_id)
            .map(|pool| PoolStats::from_pool(pool.value()))
    }

    /// Obtiene estadísticas de todos los pools
    pub fn get_all_stats(&self) -> Vec<(TenantId, PoolStats)> {
        self.pools
            .iter()
            .map(|entry| {
                let tenant_id = entry.key().clone();
                let stats = PoolStats::from_pool(entry.value());
                (tenant_id, stats)
            })
            .collect()
    }

    /// Evict pools idle (ejecutar periódicamente)
    pub async fn evict_idle_pools(&self, _min_idle_time: Duration) {
        // Nota: sin acceso a PoolState, simplemente evictamos pools con size mínimo
        let to_remove: Vec<TenantId> = self
            .pools
            .iter()
            .filter_map(|entry| {
                let size = entry.value().size();
                // Si el pool tiene el tamaño mínimo, asumimos que está idle
                if size == self.default_min_connections {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();

        for tenant_id in to_remove {
            warn!(tenant_id = %tenant_id, "Evicting idle pool");
            self.remove_pool(&tenant_id).await;
        }
    }

    /// Número de pools activos
    pub fn active_pools_count(&self) -> usize {
        self.pools.len()
    }

    /// Cierra todos los pools
    pub async fn close_all(&self) {
        let all_ids: Vec<TenantId> = self.pools.iter().map(|e| e.key().clone()).collect();

        for tenant_id in all_ids {
            self.remove_pool(&tenant_id).await;
        }

        info!("All pools closed");
    }

    /// Health check de un pool específico
    pub async fn health_check(&self, tenant_id: &TenantId) -> Result<bool, PoolError> {
        let pool = self
            .pools
            .get(tenant_id)
            .ok_or_else(|| PoolError::TenantNotFound(tenant_id.to_string()))?;

        match sqlx::query("SELECT 1").fetch_one(pool.value()).await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!(tenant_id = %tenant_id, error = %e, "Pool health check failed");
                Ok(false)
            }
        }
    }
}

impl Clone for TenantPoolManager {
    fn clone(&self) -> Self {
        Self {
            pools: Arc::clone(&self.pools),
            default_max_connections: self.default_max_connections,
            default_min_connections: self.default_min_connections,
            acquire_timeout: self.acquire_timeout,
            idle_timeout: self.idle_timeout,
        }
    }
}
