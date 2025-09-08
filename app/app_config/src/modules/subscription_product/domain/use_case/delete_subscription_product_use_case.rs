use ac_struct_back::schemas::auth::subscription_product::subscription_product::{
    DeleteSubscriptionProductError, SubscriptionProduct,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct DeleteSubscriptionProductUseCase;
#[async_trait::async_trait]
pub trait DeleteSubscriptionProductUseCasePublic {
    async fn execute(
        id: &str,
    ) -> Result<JsonAdvanced<SubscriptionProduct>, DeleteSubscriptionProductError>;
}
