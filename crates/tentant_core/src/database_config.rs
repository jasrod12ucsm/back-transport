use serde::{Deserialize, Serialize};

/// Configuración de una database específica para este microservicio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Nombre de la database (products, orders, users, etc.)
    pub name: String,
    /// Máximo de conexiones para esta database
    pub max_connections: u32,
    /// Mínimo de conexiones para esta database
    pub min_connections: u32,
}

impl DatabaseConfig {
    pub fn new(name: impl Into<String>, max_connections: u32, min_connections: u32) -> Self {
        Self {
            name: name.into(),
            max_connections,
            min_connections,
        }
    }

    /// Database con configuración por defecto
    pub fn default(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            max_connections: 10,
            min_connections: 2,
        }
    }
}
