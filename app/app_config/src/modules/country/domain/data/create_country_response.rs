use ac_struct_back::schemas::config::{
    country::country::Country, country_timezone::country_timezone::CountryTimezone,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CreateCountryResponse {
    updated: Vec<Country>,
    inserted: Vec<Country>,
    relations_inserted: Vec<CountryTimezone>,
}

impl CreateCountryResponse {
    pub fn new(
        updated: Vec<Country>,
        inserted: Vec<Country>,
        relations_inserted: Vec<CountryTimezone>,
    ) -> Self {
        Self {
            updated,
            inserted,
            relations_inserted,
        }
    }
}
