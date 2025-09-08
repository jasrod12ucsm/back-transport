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
        let errors = Vec::new();

        for (key, value) in self {
            println!("key: {}, value: {}", key, value);
            let json_value = Value::String(value.clone()); // Siempre trata el valor como String
            json_map.insert(key.clone(), json_value);
        }

        if !errors.is_empty() {
            return Err(errors);
        }
        println!("{:?}", json_map);

        serde_json::from_value(Value::Object(json_map))
            .map(Some)
            .map_err(|e| vec![e.to_string()])
    }
}
