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

/// Configuración esencial de un tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    pub id: TenantId,
    pub name: String,
    pub database_name: String, // "products", "orders", "users", etc.
    pub connection_string: String,
    pub status: TenantStatus,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl TenantConfig {
    pub fn is_active(&self) -> bool {
        self.status == TenantStatus::Active
    }

    pub fn cache_key(&self) -> String {
        format!("tenant:{}:{}", self.id, self.database_name)
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
