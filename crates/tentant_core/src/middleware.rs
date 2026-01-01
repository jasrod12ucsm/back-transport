use crate::config_resolver::TenantConfigResolver;
use crate::database_config::DatabaseConfig;
use crate::pool_manager::TenantPoolManager;
use crate::types::{TenantContext, TenantId};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;

/// Claims del JWT (solo tenant_id por ahora)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub tenant_id: String,
    #[serde(default)]
    pub exp: Option<u64>,
}

/// Datos extraídos del tenant inyectados en request
/// Contiene TODAS las databases configuradas para este microservicio
#[derive(Clone)]
pub struct TenantData {
    pub context: TenantContext,
    pools: HashMap<String, PgPool>,
}

impl TenantData {
    pub fn new(context: TenantContext, pools: HashMap<String, PgPool>) -> Self {
        Self { context, pools }
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.context.tenant_id
    }

    /// Obtiene pool por nombre de database
    pub fn pool(&self, database_name: &str) -> Result<&PgPool, PoolAccessError> {
        self.pools
            .get(database_name)
            .ok_or_else(|| PoolAccessError::DatabaseNotConfigured(database_name.to_string()))
    }

    /// Obtiene todos los pools configurados
    pub fn pools(&self) -> &HashMap<String, PgPool> {
        &self.pools
    }

    /// Obtiene la database principal (primera configurada)
    pub fn primary_pool(&self) -> Option<&PgPool> {
        self.pools.values().next()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PoolAccessError {
    #[error("Database '{0}' not configured for this service")]
    DatabaseNotConfigured(String),
}

/// Servicio para resolver tenant y obtener pools
#[derive(Clone)]
pub struct TenantResolver {
    config_resolver: Arc<TenantConfigResolver>,
    pool_manager: Arc<TenantPoolManager>,
    jwt_secret: String,
    database_configs: Vec<DatabaseConfig>,
}

impl TenantResolver {
    pub fn new(
        config_resolver: Arc<TenantConfigResolver>,
        pool_manager: Arc<TenantPoolManager>,
        jwt_secret: String,
        database_configs: Vec<DatabaseConfig>,
    ) -> Self {
        Self {
            config_resolver,
            pool_manager,
            jwt_secret,
            database_configs,
        }
    }

    /// Valida JWT y extrae tenant_id
    pub fn validate_jwt(&self, token: &str) -> Result<JwtClaims, ResolverError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| ResolverError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }

    /// Resuelve tenant desde JWT y obtiene TODOS los pools configurados
    pub async fn resolve_from_jwt(&self, token: &str) -> Result<TenantData, ResolverError> {
        // Validar JWT y extraer claims
        let claims = self.validate_jwt(token)?;

        // Parse tenant ID
        let tenant_id =
            TenantId::from_str(&claims.tenant_id).map_err(|_| ResolverError::InvalidTenantId)?;

        // Resolver TODAS las databases configuradas
        let mut pools = HashMap::new();

        for db_config in &self.database_configs {
            // Resolver config para esta database
            let config = self
                .config_resolver
                .resolve(&tenant_id, &db_config.name)
                .await
                .map_err(|e| ResolverError::ConfigResolution(e.to_string()))?;

            // Obtener pool
            let pool = self
                .pool_manager
                .get_pool(&config)
                .await
                .map_err(|e| ResolverError::PoolAcquisition(e.to_string()))?;

            pools.insert(db_config.name.clone(), pool);
        }

        // Obtener nombre del tenant (de cualquier config)
        let tenant_name = self
            .config_resolver
            .get_tenant_name(&tenant_id)
            .await
            .unwrap_or_else(|_| tenant_id.to_string());

        let context = TenantContext::new(tenant_id, tenant_name);

        Ok(TenantData::new(context, pools))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error("Missing Authorization header")]
    MissingToken,
    #[error("Invalid token format")]
    InvalidTokenFormat,
    #[error("Invalid JWT token: {0}")]
    InvalidToken(String),
    #[error("Invalid tenant ID format")]
    InvalidTenantId,
    #[error("Config resolution failed: {0}")]
    ConfigResolution(String),
    #[error("Pool acquisition failed: {0}")]
    PoolAcquisition(String),
}

// ===== Integración con ntex =====

use ntex::http::StatusCode;
use ntex::web::{ErrorRenderer, FromRequest, HttpRequest, WebResponseError};

/// Extractor para ntex que obtiene TenantData
pub struct ExtractTenant(pub TenantData);

impl ExtractTenant {
    /// Obtiene pool de una database específica
    pub fn pool(&self, database_name: &str) -> Result<&PgPool, PoolAccessError> {
        self.0.pool(database_name)
    }

    /// Obtiene todos los pools
    pub fn pools(&self) -> &HashMap<String, PgPool> {
        self.0.pools()
    }

    /// Obtiene contexto del tenant
    pub fn context(&self) -> &TenantContext {
        &self.0.context
    }
}

impl<Err: ErrorRenderer> FromRequest<Err> for ExtractTenant {
    type Error = TenantExtractionError;

    async fn from_request(
        req: &HttpRequest,
        _: &mut ntex::http::Payload,
    ) -> Result<Self, Self::Error> {
        let tenant_data = req
            .extensions()
            .get::<TenantData>()
            .cloned()
            .ok_or(TenantExtractionError::MissingToken)?;

        Ok(ExtractTenant(tenant_data))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TenantExtractionError {
    #[error("Missing Authorization header")]
    MissingToken,
    #[error("Invalid token format")]
    InvalidTokenFormat,
    #[error("Invalid JWT token")]
    InvalidToken,
    #[error("Invalid tenant ID format")]
    InvalidTenantId,
    #[error("Tenant resolution failed: {0}")]
    ResolutionFailed(String),
    #[error("Pool acquisition failed: {0}")]
    PoolFailed(String),
}

impl<Err: ErrorRenderer> WebResponseError<Err> for TenantExtractionError {
    fn error_response(&self, _: &HttpRequest) -> ntex::web::HttpResponse {
        let (status, message) = match self {
            Self::MissingToken => (StatusCode::UNAUTHORIZED, "Missing Authorization header"),
            Self::InvalidTokenFormat => (StatusCode::UNAUTHORIZED, "Invalid token format"),
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
            Self::InvalidTenantId => (StatusCode::BAD_REQUEST, "Invalid tenant ID format"),
            Self::ResolutionFailed(_) => (StatusCode::NOT_FOUND, "Tenant not found"),
            Self::PoolFailed(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Database connection unavailable",
            ),
        };

        ntex::web::HttpResponse::build(status).json(&serde_json::json!({
            "error": message,
            "details": self.to_string(),
        }))
    }
}

// Helper macros
#[macro_export]
macro_rules! with_tenant {
    ($tenant:expr) => {{ ($tenant.context(), $tenant.pools()) }};
}

#[macro_export]
macro_rules! pool {
    ($tenant:expr, $db:expr) => {{ $tenant.pool($db) }};
}

// Middleware ntex
use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web::{Error, WebRequest, WebResponse};

pub struct TenantMiddleware {
    pub resolver: TenantResolver,
}

impl TenantMiddleware {
    pub fn new(resolver: TenantResolver) -> Self {
        Self { resolver }
    }
}

impl<S> Middleware<S> for TenantMiddleware {
    type Service = TenantMiddlewareService<S>;

    fn create(&self, service: S) -> Self::Service {
        TenantMiddlewareService {
            service,
            resolver: self.resolver.clone(),
        }
    }
}

pub struct TenantMiddlewareService<S> {
    service: S,
    resolver: TenantResolver,
}

impl<S, Err> Service<WebRequest<Err>> for TenantMiddlewareService<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
    Err: ErrorRenderer,
{
    type Response = WebResponse;
    type Error = Error;

    async fn call(
        &self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        // Extraer JWT del header Authorization
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::from(TenantExtractionError::MissingToken))?;

        // Extraer token del formato "Bearer <token>"
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| Error::from(TenantExtractionError::InvalidTokenFormat))?;

        // Resolver tenant desde JWT (resuelve TODAS las databases configuradas)
        let tenant_data = self
            .resolver
            .resolve_from_jwt(token)
            .await
            .map_err(|e| match e {
                ResolverError::MissingToken => Error::from(TenantExtractionError::MissingToken),
                ResolverError::InvalidTokenFormat => {
                    Error::from(TenantExtractionError::InvalidTokenFormat)
                }
                ResolverError::InvalidToken(_) => Error::from(TenantExtractionError::InvalidToken),
                ResolverError::InvalidTenantId => {
                    Error::from(TenantExtractionError::InvalidTenantId)
                }
                ResolverError::ConfigResolution(msg) => {
                    Error::from(TenantExtractionError::ResolutionFailed(msg))
                }
                ResolverError::PoolAcquisition(msg) => {
                    Error::from(TenantExtractionError::PoolFailed(msg))
                }
            })?;

        // Inyectar en extensions
        req.extensions_mut().insert(tenant_data);

        // Continuar con el request
        ctx.call(&self.service, req).await
    }
}
