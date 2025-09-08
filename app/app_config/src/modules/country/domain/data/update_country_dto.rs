use ac_struct_back::common::enums::status_enum::StatusEnum;
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct UpdateCountryDto {
    pub status: Option<StatusEnum>,
}
