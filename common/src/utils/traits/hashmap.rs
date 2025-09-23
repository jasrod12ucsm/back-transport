use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub trait HashMapToStruct {
    fn try_from_hashmap<T>(&self) -> Result<Option<T>, Vec<String>>
    where
        T: DeserializeOwned + Default;
}

impl HashMapToStruct for HashMap<String, String> {
    fn try_from_hashmap<T>(&self) -> Result<Option<T>, Vec<String>>
    where
        T: DeserializeOwned + Default,
    {
        if self.is_empty() {
            return Ok(None);
        }

        let mut json_map = Map::new();
        let mut errors = Vec::new();

        for (key, value) in self {
            // Intentamos interpretar el valor como JSON
            let json_value = match serde_json::from_str::<Value>(value) {
                Ok(v) => v, // Si es un número, array, bool o null, se mantiene
                Err(_) => Value::String(value.clone()), // Si falla, lo dejamos como String
            };
            json_map.insert(key.clone(), json_value);
        }
        println!("json_map: {:?}", json_map);

        if !errors.is_empty() {
            return Err(errors);
        }

        serde_json::from_value(Value::Object(json_map))
            .map(Some)
            .map_err(|e| vec![e.to_string()])
    }
}
