use std::borrow::BorrowMut;
use std::sync::Arc;

use config;
use config::{builder::DefaultState, Config, ConfigBuilder, ConfigError, File, Map, Source, Value};
use serde::Deserialize;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub struct Env {
    config: Config,
}

impl Env {
    fn new() -> Self {
        Self {
            config: ConfigBuilder::<DefaultState>::default()
                .add_source(File::with_name("config.json"))
                .build()
                .expect("Error al cargar configuración"),
        }
    }

    pub fn get<'de, T: Deserialize<'de>>(&self, key: &str) -> Result<T, ConfigError> {
        return self.config.get::<T>(key);
    }

    pub fn get_string(&self, key: &str) -> Result<String, ConfigError> {
        return self.config.get::<String>(key);
    }

    pub fn get_int(&self, key: &str) -> Result<i64, ConfigError> {
        return self.config.get::<i64>(key);
    }

    pub fn get_float(&self, key: &str) -> Result<f64, ConfigError> {
        return self.config.get::<f64>(key);
    }

    pub fn get_bool(&self, key: &str) -> Result<bool, ConfigError> {
        return self.config.get::<bool>(key);
    }

    pub fn get_table(&self, key: &str) -> Result<Map<String, Value>, ConfigError> {
        self.config.get_table(key)
    }

    pub fn get_array(&self, key: &str) -> Result<Vec<Value>, ConfigError> {
        self.config.get_array(key)
    }

    /// Attempt to deserialize the entire configuration into the requested type.
    pub fn try_deserialize<'de, T: Deserialize<'de>>(self) -> Result<T, ConfigError> {
        self.config.try_deserialize()
    }

    #[deprecated(since = "0.7.0", note = "please use 'try_deserialize' instead")]
    pub fn deserialize<'de, T: Deserialize<'de>>(self) -> Result<T, ConfigError> {
        self.try_deserialize()
    }
    pub fn clone_into_box(&mut self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.config.borrow_mut().clone())
    }

    pub fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        self.config.cache.clone().into_table()
    }
}

lazy_static!(
    pub static ref ENV:Arc<Env>=Arc::new(Env::new()); 
);

