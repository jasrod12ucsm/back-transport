use ac_struct_back::schemas::config::timezone::timezone::Timezone;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CreateTimezoneDto {
    pub timezones: Vec<TimezoneDto>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TimezoneDto {
    #[serde(rename = "zoneName")]
    pub zone_name: String,
    #[serde(rename = "gmtOffset")]
    pub gmt_offset: i32,
    #[serde(rename = "gmtOffsetName")]
    pub gmt_offset_name: String,
    #[serde(rename = "tzName")]
    pub tz_name: String,
}

impl From<TimezoneDto> for Timezone {
    fn from(dto: TimezoneDto) -> Self {
        Timezone {
            id: None,
            name: dto.zone_name,
            gmt_offset: dto.gmt_offset,
            gmt_offset_name: dto.gmt_offset_name,
            ..Default::default()
        }
    }
}
