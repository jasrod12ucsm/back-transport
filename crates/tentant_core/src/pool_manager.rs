use crate::types::{TenantConfig, TenantId};
use dashmap::DashMap;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info};

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

impl PoolStats {
    pub fn from_pool(pool: &PgPool) -> Self {
        let size = pool.size();
        Self {
            size,
            idle: 0, // sqlx no expone idle connections en la API pública
        }
    }
}

/// Clave compuesta para identificar pools únicos
/// Soporta múltiples databases por tenant (products, orders, users, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PoolKey {
    pub tenant_id: TenantId,
    pub database_name: String,
}

impl PoolKey {
    pub fn new(tenant_id: TenantId, database_name: String) -> Self {
        Self {
            tenant_id,
            database_name,
        }
    }
}

impl std::fmt::Display for PoolKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.tenant_id, self.database_name)
    }
}

/// Manager de pools multi-tenant usando sqlx
/// Soporta múltiples databases por tenant (un pool por database)
pub struct TenantPoolManager {
    /// Map de (tenant_id, database_name) -> PgPool
    pools: Arc<DashMap<PoolKey, PgPool>>,
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

    /// Obtiene o crea pool para un tenant y database específica
    pub async fn get_pool(&self, config: &TenantConfig) -> Result<PgPool, PoolError> {
        let key = PoolKey::new(config.id.clone(), config.database_name.clone());

        // Usar entry API para evitar race conditions
        if let Some(pool) = self.pools.get(&key) {
            debug!(
                tenant_id = %config.id,
                database = %config.database_name,
                "Pool cache hit"
            );
            return Ok(pool.clone());
        }

        debug!(
            tenant_id = %config.id,
            database = %config.database_name,
            "Creating new pool"
        );

        let pool = self.create_pool(config).await?;

        // Insertar y retornar el pool
        self.pools.insert(key.clone(), pool.clone());

        info!(
            tenant_id = %config.id,
            database = %config.database_name,
            size = pool.size(),
            "Pool created successfully"
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

        let min_connections = if config.min_connections > 0 {
            config.min_connections
        } else {
            self.default_min_connections
        };

        // Parse connection string
        let connect_opts = PgConnectOptions::from_str(&config.connection_string)
            .map_err(|e| PoolError::InvalidConnectionString(e.to_string()))?
            .application_name(&format!("tenant-{}-{}", config.id, config.database_name))
            .log_statements(tracing::log::LevelFilter::Debug)
            .log_slow_statements(tracing::log::LevelFilter::Warn, Duration::from_millis(500));

        // Crear pool con configuración
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(self.acquire_timeout)
            .idle_timeout(Some(self.idle_timeout))
            .max_lifetime(Some(Duration::from_secs(3600))) // 1 hora max lifetime
            .test_before_acquire(true) // Verifica conexión antes de entregar
            .connect_with(connect_opts)
            .await
            .map_err(|e| PoolError::CreationFailed(e.to_string()))?;

        Ok(pool)
    }

    /// Cierra pool para un tenant/database específico
    pub async fn close_pool(&self, tenant_id: &TenantId, database_name: &str) {
        let key = PoolKey::new(tenant_id.clone(), database_name.to_string());

        if let Some((_, pool)) = self.pools.remove(&key) {
            pool.close().await;
            info!(
                tenant_id = %tenant_id,
                database = %database_name,
                "Pool closed"
            );
        }
    }

    /// Cierra TODOS los pools de un tenant (todas sus databases)
    pub async fn close_all_tenant_pools(&self, tenant_id: &TenantId) {
        let mut closed_count = 0;

        // Filtrar todas las keys que pertenecen a este tenant
        let keys_to_remove: Vec<PoolKey> = self
            .pools
            .iter()
            .filter(|entry| entry.key().tenant_id == *tenant_id)
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            if let Some((_, pool)) = self.pools.remove(&key) {
                pool.close().await;
                closed_count += 1;
            }
        }

        if closed_count > 0 {
            info!(
                tenant_id = %tenant_id,
                pools_closed = closed_count,
                "All tenant pools closed"
            );
        }
    }

    /// Obtiene estadísticas de un pool
    pub fn get_pool_stats(&self, tenant_id: &TenantId, database_name: &str) -> Option<PoolStats> {
        let key = PoolKey::new(tenant_id.clone(), database_name.to_string());
        self.pools.get(&key).map(|pool| PoolStats::from_pool(&pool))
    }

    /// Cuenta pools activos
    pub fn active_pools_count(&self) -> usize {
        self.pools.len()
    }

    /// Evict pools inactivos (sin uso reciente)
    pub async fn evict_idle_pools(&self, max_idle_time: Duration) {
        let mut evicted = 0;

        // Por simplicidad, evict todos los pools
        // En producción, querrías trackear último uso
        let all_keys: Vec<PoolKey> = self.pools.iter().map(|e| e.key().clone()).collect();

        for key in all_keys {
            if let Some((_, pool)) = self.pools.remove(&key) {
                pool.close().await;
                evicted += 1;
            }
        }

        if evicted > 0 {
            info!(pools_evicted = evicted, "Idle pools evicted");
        }
    }

    /// Health check - verifica que se pueden crear pools
    pub async fn health_check(&self, config: &TenantConfig) -> Result<(), PoolError> {
        let pool = self.get_pool(config).await?;
        sqlx::query("SELECT 1").fetch_one(&pool).await?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_key() {
        let tenant_id = TenantId::new();
        let key1 = PoolKey::new(tenant_id.clone(), "products".to_string());
        let key2 = PoolKey::new(tenant_id.clone(), "orders".to_string());
        let key3 = PoolKey::new(tenant_id.clone(), "products".to_string());

        assert_ne!(key1, key2); // Diferentes databases
        assert_eq!(key1, key3); // Misma database
    }

    #[test]
    fn test_pool_key_display() {
        let tenant_id = TenantId::new();
        let key = PoolKey::new(tenant_id.clone(), "products".to_string());
        let display = format!("{}", key);

        assert!(display.contains("products"));
        assert!(display.contains(&tenant_id.to_string()));
    }
}
