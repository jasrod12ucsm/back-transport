use crate::crypto;
use crate::types::{TenantConfig, TenantId, TenantStatus};
use moka::future::Cache;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("Tenant not found: {0}")]
    TenantNotFound(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    #[error("Tenant is not active: {0}")]
    TenantNotActive(String),
}

/// Resuelve configuración de tenant con cache híbrido opcional L1 → L2 → L3
pub struct TenantConfigResolver {
    /// L1: Cache local in-memory (opcional)
    local_cache: Option<Cache<TenantId, Arc<TenantConfig>>>,
    /// L2: Redis distributed cache (opcional)
    redis: Option<ConnectionManager>,
    /// L3: PostgreSQL catalog database (siempre - source of truth)
    catalog_db: PgPool,
    /// Clave de encriptación para connection strings
    encryption_key: [u8; 32],
    /// TTL para cache L2
    l2_ttl_seconds: u64,
    /// Nombre de la database para este servicio
    database_name: String,
}

impl TenantConfigResolver {
    /// Crea un builder para configurar el resolver
    pub fn builder(
        catalog_db: PgPool,
        encryption_key: [u8; 32],
        database_name: String,
    ) -> TenantConfigResolverBuilder {
        TenantConfigResolverBuilder {
            catalog_db,
            encryption_key,
            database_name,
            enable_l1: false,
            enable_l2: false,
            redis_url: None,
            l1_max_capacity: 1000,
            l1_ttl_seconds: 60,
            l1_tti_seconds: 30,
            l2_ttl_seconds: 900,
        }
    }

    /// Resuelve config para un tenant y database específica
    pub async fn resolve(
        &self,
        tenant_id: &TenantId,
        database_name: &str,
    ) -> Result<Arc<TenantConfig>, ResolverError> {
        // L1: Check local cache (si está habilitado)
        // Clave de cache incluye database_name
        let cache_key = format!("{}:{}", tenant_id, database_name);

        if let Some(cache) = &self.local_cache {
            // Nota: Moka usa TenantId como key, pero podríamos usar un wrapper
            // Por simplicidad, solo checamos L2 y L3 cuando database_name != self.database_name
            if database_name == self.database_name {
                if let Some(config) = cache.get(tenant_id).await {
                    debug!(
                        tenant_id = %tenant_id,
                        database = %database_name,
                        "L1 cache hit"
                    );
                    return Ok(config);
                }
            }
        }

        // L2: Check Redis (si está habilitado)
        if let Some(redis) = &self.redis {
            let redis_key = format!("tenant:{}:{}:config", tenant_id, database_name);
            let mut redis_conn = redis.clone();

            match redis_conn.get::<_, Option<String>>(&redis_key).await {
                Ok(Some(json)) => {
                    if let Ok(config) = serde_json::from_str::<TenantConfig>(&json) {
                        debug!(
                            tenant_id = %tenant_id,
                            database = %database_name,
                            "L2 cache hit"
                        );
                        let config = Arc::new(config);

                        // Populate L1 solo si es la database principal
                        if database_name == self.database_name {
                            if let Some(cache) = &self.local_cache {
                                cache.insert(tenant_id.clone(), config.clone()).await;
                            }
                        }
                        return Ok(config);
                    }
                }
                Ok(None) => debug!(
                    tenant_id = %tenant_id,
                    database = %database_name,
                    "L2 cache miss"
                ),
                Err(e) => warn!(
                    tenant_id = %tenant_id,
                    database = %database_name,
                    error = %e,
                    "L2 cache error"
                ),
            }
        }

        // L3: Query database (siempre)
        debug!(
            tenant_id = %tenant_id,
            database = %database_name,
            "L3 database lookup"
        );
        let config = self.fetch_from_db(tenant_id, database_name).await?;
        let config = Arc::new(config);

        // Populate caches (los que estén habilitados)
        self.populate_caches(tenant_id, database_name, &config)
            .await;

        Ok(config)
    }

    /// Obtiene el nombre del tenant (sin cargar toda la config)
    pub async fn get_tenant_name(&self, tenant_id: &TenantId) -> Result<String, ResolverError> {
        let row = sqlx::query(
            r#"
            SELECT name FROM tenants 
            WHERE id = $1
            LIMIT 1
            "#,
        )
        .bind(*tenant_id.as_uuid())
        .fetch_optional(&self.catalog_db)
        .await?
        .ok_or_else(|| ResolverError::TenantNotFound(tenant_id.to_string()))?;

        let name: String = row.try_get("name")?;
        Ok(name)
    }

    /// Obtiene config desde PostgreSQL para una database específica
    async fn fetch_from_db(
        &self,
        tenant_id: &TenantId,
        database_name: &str,
    ) -> Result<TenantConfig, ResolverError> {
        let row = sqlx::query(
            r#"
            SELECT 
                id,
                name,
                connection_string_encrypted,
                status,
                max_connections,
                min_connections
            FROM tenants 
            WHERE id = $1 AND database_name = $2
            "#,
        )
        .bind(*tenant_id.as_uuid())
        .bind(database_name)
        .fetch_optional(&self.catalog_db)
        .await?
        .ok_or_else(|| ResolverError::TenantNotFound(tenant_id.to_string()))?;

        // Extraer valores manualmente
        let id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let connection_string_encrypted: Vec<u8> = row.try_get("connection_string_encrypted")?;
        let status_str: String = row.try_get("status")?;
        let max_connections: Option<i32> = row.try_get("max_connections")?;
        let min_connections: Option<i32> = row.try_get("min_connections")?;

        // Parsear status
        let status = match status_str.as_str() {
            "provisioning" => TenantStatus::Provisioning,
            "active" => TenantStatus::Active,
            "suspended" => TenantStatus::Suspended,
            "deactivated" => TenantStatus::Deactivated,
            _ => {
                return Err(ResolverError::DatabaseError(sqlx::Error::Decode(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid status: {}", status_str),
                    ),
                ))));
            }
        };

        // Verificar que esté activo
        if status != TenantStatus::Active {
            return Err(ResolverError::TenantNotActive(tenant_id.to_string()));
        }

        // Desencriptar connection string
        let connection_string = crypto::decrypt(&connection_string_encrypted, &self.encryption_key)
            .map_err(|e| ResolverError::DecryptionError(e.to_string()))?;

        Ok(TenantConfig {
            id: TenantId::from_uuid(id),
            name,
            database_name: database_name.to_string(),
            connection_string,
            status,
            max_connections: max_connections.unwrap_or(10) as u32,
            min_connections: min_connections.unwrap_or(2) as u32,
        })
    }

    /// Puebla caches habilitados con una config
    async fn populate_caches(
        &self,
        tenant_id: &TenantId,
        database_name: &str,
        config: &Arc<TenantConfig>,
    ) {
        // L1 (solo si es la database principal)
        if database_name == self.database_name {
            if let Some(cache) = &self.local_cache {
                cache.insert(tenant_id.clone(), config.clone()).await;
            }
        }

        // L2 (si está habilitado)
        if let Some(redis) = &self.redis {
            let redis_key = format!("tenant:{}:{}:config", tenant_id, database_name);
            let mut redis_conn = redis.clone();

            if let Ok(json) = serde_json::to_string(config.as_ref()) {
                if let Err(e) = redis_conn
                    .set_ex::<_, _, String>(&redis_key, &json, self.l2_ttl_seconds)
                    .await
                {
                    warn!(
                        tenant_id = %tenant_id,
                        database = %database_name,
                        error = %e,
                        "Failed to populate L2 cache"
                    );
                }
            }
        }
    }

    /// Invalida cache para un tenant y database específica
    pub async fn invalidate(&self, tenant_id: &TenantId, database_name: &str) {
        // L1 (solo si es la principal)
        if database_name == self.database_name {
            if let Some(cache) = &self.local_cache {
                cache.invalidate(tenant_id).await;
            }
        }

        // L2
        if let Some(redis) = &self.redis {
            let redis_key = format!("tenant:{}:{}:config", tenant_id, database_name);
            let mut redis_conn = redis.clone();

            if let Err(e) = redis_conn.del::<&str, i32>(&redis_key).await {
                warn!(
                    tenant_id = %tenant_id,
                    database = %database_name,
                    error = %e,
                    "Failed to invalidate L2 cache"
                );
            }
        }

        info!(
            tenant_id = %tenant_id,
            database = %database_name,
            "Cache invalidated"
        );
    }

    /// Invalida cache de múltiples tenants
    pub async fn invalidate_many(&self, tenant_ids: &[(TenantId, String)]) {
        for (tenant_id, database_name) in tenant_ids {
            self.invalidate(tenant_id, database_name).await;
        }
    }

    /// Pre-carga config en cache
    pub async fn preload(
        &self,
        tenant_id: &TenantId,
        database_name: &str,
    ) -> Result<(), ResolverError> {
        let config = self.fetch_from_db(tenant_id, database_name).await?;
        self.populate_caches(tenant_id, database_name, &Arc::new(config))
            .await;
        Ok(())
    }

    /// Estadísticas del cache L1
    pub fn cache_stats(&self) -> (u64, u64) {
        if let Some(cache) = &self.local_cache {
            (cache.entry_count(), cache.weighted_size())
        } else {
            (0, 0)
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), ResolverError> {
        // Check Redis (si está habilitado)
        if let Some(redis) = &self.redis {
            let mut redis_conn = redis.clone();
            let _: () = redis_conn.ping().await?;
        }

        // Check Catalog DB (siempre)
        sqlx::query("SELECT 1").fetch_one(&self.catalog_db).await?;

        Ok(())
    }
}

impl Clone for TenantConfigResolver {
    fn clone(&self) -> Self {
        Self {
            local_cache: self.local_cache.clone(),
            redis: self.redis.clone(),
            catalog_db: self.catalog_db.clone(),
            encryption_key: self.encryption_key,
            l2_ttl_seconds: self.l2_ttl_seconds,
            database_name: self.database_name.clone(),
        }
    }
}

/// Builder para TenantConfigResolver
pub struct TenantConfigResolverBuilder {
    catalog_db: PgPool,
    encryption_key: [u8; 32],
    database_name: String,
    enable_l1: bool,
    enable_l2: bool,
    redis_url: Option<String>,
    l1_max_capacity: u64,
    l1_ttl_seconds: u64,
    l1_tti_seconds: u64,
    l2_ttl_seconds: u64,
}

impl TenantConfigResolverBuilder {
    /// Habilita cache local (Moka) con configuración personalizada
    pub fn with_local_cache(
        mut self,
        max_capacity: u64,
        ttl_seconds: u64,
        tti_seconds: u64,
    ) -> Self {
        self.enable_l1 = true;
        self.l1_max_capacity = max_capacity;
        self.l1_ttl_seconds = ttl_seconds;
        self.l1_tti_seconds = tti_seconds;
        self
    }

    /// Habilita Redis con TTL personalizado
    pub fn with_redis(mut self, redis_url: String, ttl_seconds: u64) -> Self {
        self.enable_l2 = true;
        self.redis_url = Some(redis_url);
        self.l2_ttl_seconds = ttl_seconds;
        self
    }

    /// Construye el resolver
    pub async fn build(self) -> Result<TenantConfigResolver, ResolverError> {
        // L1: Crear cache local si está habilitado
        let local_cache = if self.enable_l1 {
            Some(
                Cache::builder()
                    .max_capacity(self.l1_max_capacity)
                    .time_to_live(Duration::from_secs(self.l1_ttl_seconds))
                    .time_to_idle(Duration::from_secs(self.l1_tti_seconds))
                    .build(),
            )
        } else {
            None
        };

        // L2: Crear conexión Redis si está habilitado
        let redis = if self.enable_l2 {
            let url = self
                .redis_url
                .expect("Redis URL required when L2 cache is enabled");
            let client = redis::Client::open(url.as_str())?;
            Some(ConnectionManager::new(client).await?)
        } else {
            None
        };

        Ok(TenantConfigResolver {
            local_cache,
            redis,
            catalog_db: self.catalog_db,
            encryption_key: self.encryption_key,
            l2_ttl_seconds: self.l2_ttl_seconds,
            database_name: self.database_name,
        })
    }
}
