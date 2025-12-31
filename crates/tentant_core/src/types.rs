use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Identificador único de tenant
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(Uuid);

impl TenantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TenantId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Estado del tenant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "tenant_status", rename_all = "lowercase")]
pub enum TenantStatus {
    Provisioning,
    Active,
    Suspended,
    Deactivated,
}

/// Configuración completa de un tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    pub id: TenantId,
    pub name: String,
    pub slug: String,
    pub connection_string: String,
    pub min_connections: u32,
    pub status: TenantStatus,
    pub max_connections: u32,
    pub region: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neon_project_id: Option<String>,
}

impl TenantConfig {
    pub fn is_active(&self) -> bool {
        self.status == TenantStatus::Active
    }

    pub fn cache_key(&self) -> String {
        format!("tenant:{}", self.id)
    }
}

/// Contexto de tenant para requests
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub tenant_name: String,
}

impl TenantContext {
    pub fn new(tenant_id: TenantId, tenant_name: String) -> Self {
        Self {
            tenant_id,
            tenant_name,
        }
    }
}

/// Información de Neon project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeonProject {
    pub id: String,
    pub name: String,
    pub region_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub database_host: String,
    pub database_name: String,
}

/// Quotas de Neon por tier
#[derive(Debug, Clone)]
pub struct NeonQuota {
    pub compute_time_seconds: i64,
    pub storage_bytes: i64,
    pub autoscaling_min_cu: f32,
    pub autoscaling_max_cu: f32,
    pub suspend_timeout_seconds: u32,
}
