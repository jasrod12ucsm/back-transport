use ac_struct_back::schemas::config::timezone::timezone::{CreateTimezoneError, Timezone};
use common::utils::ntex_private::extractors::multipart_extractor::MultipartData;

use crate::utils::charge_models::void_struct::VoidStruct;
pub struct CreateTimezoneUseCase;
#[async_trait::async_trait]
pub trait CreateTimezoneUseCaseTrait {
    async fn execute(dto: MultipartData<VoidStruct>) -> Result<Vec<Timezone>, CreateTimezoneError>;
}
