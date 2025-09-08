use ac_struct_back::common::enums::user_type::UserTypeEnum;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegisterDto {
    pub email: String,
    pub name: String,
    pub surnames: String,
    pub country_code: String,
    pub user_type: UserTypeEnum,
    pub hash: String,
}
