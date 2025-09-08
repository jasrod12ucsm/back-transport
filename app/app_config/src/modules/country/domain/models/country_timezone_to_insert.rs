use crate::modules::country::domain::data::create_country_timezone_dto::CountryTimezoneDto;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct InsertCountryTimezone {
    pub id_key: Option<String>,
    pub iso2: String,
    pub timezones: Vec<CountryTimezoneDto>,
}
