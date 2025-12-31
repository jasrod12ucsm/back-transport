use crate::config_resolver::TenantConfigResolver;
use crate::pool_manager::TenantPoolManager;
use crate::types::{TenantContext, TenantId};
use sqlx::PgPool;
use std::sync::Arc;

/// Datos extraídos del tenant inyectados en request
#[derive(Clone)]
pub struct TenantData {
    pub context: TenantContext,
    pub pool: PgPool,
}

impl TenantData {
    pub fn new(context: TenantContext, pool: PgPool) -> Self {
        Self { context, pool }
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.context.tenant_id
    }
}

/// Servicio para resolver tenant y obtener pool
#[derive(Clone)]
pub struct TenantResolver {
    config_resolver: Arc<TenantConfigResolver>,
    pool_manager: Arc<TenantPoolManager>,
}

impl TenantResolver {
    pub fn new(
        config_resolver: Arc<TenantConfigResolver>,
        pool_manager: Arc<TenantPoolManager>,
    ) -> Self {
        Self {
            config_resolver,
            pool_manager,
        }
    }

    /// Resuelve tenant desde header y obtiene pool
    pub async fn resolve_from_header(
        &self,
        tenant_id_str: &str,
    ) -> Result<TenantData, ResolverError> {
        // Parse tenant ID
        let tenant_id =
            TenantId::from_str(tenant_id_str).map_err(|_| ResolverError::InvalidTenantId)?;

        // Resolver config (con cache L1/L2/L3)
        let config = self
            .config_resolver
            .resolve(&tenant_id)
            .await
            .map_err(|e| ResolverError::ConfigResolution(e.to_string()))?;

        // Obtener pool
        let pool = self
            .pool_manager
            .get_pool(&config)
            .await
            .map_err(|e| ResolverError::PoolAcquisition(e.to_string()))?;

        let context = TenantContext::new(tenant_id, config.name.clone());

        Ok(TenantData::new(context, pool))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error("Missing tenant ID header")]
    MissingTenantId,
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

impl<Err: ErrorRenderer> FromRequest<Err> for ExtractTenant {
    type Error = TenantExtractionError;

    async fn from_request(
        req: &HttpRequest,
        _: &mut ntex::http::Payload,
    ) -> Result<Self, Self::Error> {
        // Intentar obtener TenantData de las extensions (debe ser inyectado por middleware)
        let tenant_data = req
            .extensions()
            .get::<TenantData>()
            .cloned()
            .ok_or(TenantExtractionError::MissingTenantId)?;

        Ok(ExtractTenant(tenant_data))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TenantExtractionError {
    #[error("Missing X-Tenant-ID header")]
    MissingTenantId,
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
            Self::MissingTenantId => (StatusCode::BAD_REQUEST, "Missing X-Tenant-ID header"),
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

// Helper macro para usar en handlers
#[macro_export]
macro_rules! with_tenant {
    ($tenant:expr) => {{
        let tenant_data = $tenant.0;
        (tenant_data.context, tenant_data.pool)
    }};
}

// Middleware ntex para resolver tenant de forma async
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
        // Extraer tenant_id del header
        let tenant_id = req
            .headers()
            .get("X-Tenant-ID")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::from(TenantExtractionError::MissingTenantId))?;

        // Resolver tenant
        let tenant_data =
            self.resolver
                .resolve_from_header(tenant_id)
                .await
                .map_err(|e| match e {
                    ResolverError::MissingTenantId => {
                        Error::from(TenantExtractionError::MissingTenantId)
                    }
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
