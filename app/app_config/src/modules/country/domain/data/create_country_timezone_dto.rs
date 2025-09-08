#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CreateCountryTimezoneDto {
    pub iso2: String,
    pub timezones: Vec<CountryTimezoneDto>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CountryTimezoneDto {
    #[serde(rename = "zoneName")]
    pub zone_name: String,
    #[serde(rename = "gmtOffset")]
    pub gmt_offset: i32,
    #[serde(rename = "gmtOffsetName")]
    pub gmt_offset_name: String,
    #[serde(rename = "tzName")]
    pub tz_name: String,
}
