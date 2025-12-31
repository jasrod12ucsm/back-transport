//! # Tenant Core
//!
//! Librería compartida para manejo de multi-tenancy con:
//! - Pool manager con sqlx para múltiples databases
//! - Config resolver con cache híbrido L1/L2/L3
//! - Event system con NATS JetStream
//! - Middleware para ntex
//! - Encriptación de connection strings

pub mod config_resolver;
pub mod crypto;
pub mod events;
pub mod middleware;
pub mod pool_manager;
pub mod types;

// Re-exports para conveniencia
pub use config_resolver::{ResolverError, TenantConfigResolver, TenantConfigResolverBuilder};
pub use crypto::{decrypt, decrypt_base64, derive_key_from_password, encrypt, encrypt_base64};
pub use events::{
    TenantCreatedEvent, TenantDeactivatedEvent, TenantEvent, TenantEventHandler,
    TenantEventPublisher, TenantEventSubscriber, TenantUpdatedEvent, spawn_subscriber,
};
pub use middleware::{ExtractTenant, TenantData, TenantMiddleware, TenantResolver};
pub use pool_manager::{PoolError, PoolStats, TenantPoolManager};
pub use types::{NeonProject, TenantConfig, TenantContext, TenantId, TenantStatus};

/// Versión del crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Builder para configurar tenant-core
pub struct TenantCoreBuilder {
    catalog_db: sqlx::PgPool,
    encryption_key: [u8; 32],
    // Pool manager config
    max_connections: u32,
    min_connections: u32,
    // Cache config
    enable_l1_cache: bool,
    l1_max_capacity: u64,
    l1_ttl_seconds: u64,
    l1_tti_seconds: u64,
    enable_l2_cache: bool,
    redis_url: Option<String>,
    l2_ttl_seconds: u64,
}

impl TenantCoreBuilder {
    /// Crea un builder con catalog DB y clave de encriptación
    pub fn new(catalog_db: sqlx::PgPool, encryption_key: [u8; 32]) -> Self {
        Self {
            catalog_db,
            encryption_key,
            max_connections: 10,
            min_connections: 2,
            enable_l1_cache: false,
            l1_max_capacity: 1000,
            l1_ttl_seconds: 60,
            l1_tti_seconds: 30,
            enable_l2_cache: false,
            redis_url: None,
            l2_ttl_seconds: 900,
        }
    }

    /// Deriva clave de encriptación desde variable de entorno
    ///
    /// # Variables de entorno
    /// - `ENCRYPTION_PASSWORD`: Password para derivar la clave (recomendado)
    /// - `ENCRYPTION_KEY`: Clave en base64 (fallback)
    ///
    /// # Ejemplo
    /// ```bash
    /// export ENCRYPTION_PASSWORD="my-super-secret-password"
    /// ```
    pub fn with_encryption_from_env(mut self) -> Result<Self, std::env::VarError> {
        // Intentar obtener password para derivar clave
        if let Ok(password) = std::env::var("ENCRYPTION_PASSWORD") {
            // Usar una salt fija derivada del nombre del proyecto
            let salt = b"tenant-core-v1-salt-2024";
            self.encryption_key = derive_key_from_password(&password, salt);
            return Ok(self);
        }

        // Fallback: intentar obtener clave directamente en base64
        if let Ok(key_b64) = std::env::var("ENCRYPTION_KEY") {
            use base64::{Engine, engine::general_purpose::STANDARD};
            let key_bytes = STANDARD
                .decode(key_b64)
                .map_err(|_| std::env::VarError::NotPresent)?;

            if key_bytes.len() != 32 {
                return Err(std::env::VarError::NotPresent);
            }

            self.encryption_key.copy_from_slice(&key_bytes);
            return Ok(self);
        }

        Err(std::env::VarError::NotPresent)
    }

    /// Configura conexiones del pool manager
    pub fn pool_config(mut self, max_connections: u32, min_connections: u32) -> Self {
        self.max_connections = max_connections;
        self.min_connections = min_connections;
        self
    }

    /// Habilita cache local (Moka/L1)
    pub fn with_local_cache(
        mut self,
        max_capacity: u64,
        ttl_seconds: u64,
        tti_seconds: u64,
    ) -> Self {
        self.enable_l1_cache = true;
        self.l1_max_capacity = max_capacity;
        self.l1_ttl_seconds = ttl_seconds;
        self.l1_tti_seconds = tti_seconds;
        self
    }

    /// Habilita Redis (L2)
    pub fn with_redis_cache(mut self, redis_url: String, ttl_seconds: u64) -> Self {
        self.enable_l2_cache = true;
        self.redis_url = Some(redis_url);
        self.l2_ttl_seconds = ttl_seconds;
        self
    }

    /// Construye TenantCore
    pub async fn build(self) -> Result<TenantCore, config_resolver::ResolverError> {
        // Construir config resolver con builder pattern
        let mut resolver_builder =
            TenantConfigResolver::builder(self.catalog_db.clone(), self.encryption_key);

        if self.enable_l1_cache {
            resolver_builder = resolver_builder.with_local_cache(
                self.l1_max_capacity,
                self.l1_ttl_seconds,
                self.l1_tti_seconds,
            );
        }

        if self.enable_l2_cache {
            let redis_url = self
                .redis_url
                .expect("Redis URL required when L2 cache is enabled");
            resolver_builder = resolver_builder.with_redis(redis_url, self.l2_ttl_seconds);
        }

        let config_resolver = resolver_builder.build().await?;

        let pool_manager = TenantPoolManager::new(
            self.max_connections,
            self.min_connections,
            30,  // acquire timeout
            600, // idle timeout
        );

        Ok(TenantCore {
            config_resolver: std::sync::Arc::new(config_resolver),
            pool_manager: std::sync::Arc::new(pool_manager),
        })
    }
}

/// Instancia principal de tenant-core
#[derive(Clone)]
pub struct TenantCore {
    pub config_resolver: std::sync::Arc<TenantConfigResolver>,
    pub pool_manager: std::sync::Arc<TenantPoolManager>,
}

impl TenantCore {
    pub fn resolver(&self) -> TenantResolver {
        TenantResolver::new(self.config_resolver.clone(), self.pool_manager.clone())
    }

    pub fn middleware(&self) -> TenantMiddleware {
        TenantMiddleware::new(self.resolver())
    }

    pub async fn health_check(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.config_resolver.health_check().await?;
        Ok(())
    }

    pub fn stats(&self) -> CoreStats {
        let (cache_entries, cache_size) = self.config_resolver.cache_stats();
        let active_pools = self.pool_manager.active_pools_count();

        CoreStats {
            cache_entries,
            cache_size,
            active_pools,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoreStats {
    pub cache_entries: u64,
    pub cache_size: u64,
    pub active_pools: usize,
}
