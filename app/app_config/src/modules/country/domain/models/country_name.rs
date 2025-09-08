use surrealdb::RecordId;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CountryName {
    pub id: RecordId,
    pub cca2: String,
    pub timezone: Vec<TimezoneCountryProyection>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TimezoneCountryProyection {
    id: RecordId,
    name: String,
}

impl TimezoneCountryProyection {
    pub fn new(table: String, key: String, name: String) -> Self {
        Self {
            id: RecordId::from_table_key(table, key),
            name,
        }
    }
    pub fn id(&self) -> String {
        self.id.to_string()
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}
