use ac_struct_back::{
    schemas::auth::subscription_product::subscription_product::{
        SubscriptionProduct, UpdateSubscriptionProductError,
        updatesubscriptionproductdtosubscriptionproduct::UpdateSubscriptionProductDto,
    },
    utils::domain::front_query::{UpdateRequestBuilderFront, UpdateRequestFront},
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
pub struct UpdateSubscriptionProductUseCase;

#[async_trait::async_trait]
pub trait UpdateSubscriptionProductUseCaseTrait {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<SubscriptionProduct>,
        dto: JsonAdvanced<UpdateSubscriptionProductDto>,
    ) -> Result<JsonAdvanced<Vec<SubscriptionProduct>>, UpdateSubscriptionProductError>;
}
