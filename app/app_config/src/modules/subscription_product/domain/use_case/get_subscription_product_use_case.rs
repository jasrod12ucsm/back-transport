use ac_struct_back::{
    schemas::auth::subscription_product::subscription_product::{
        GetSubscriptionProductError, SubscriptionProduct,
    },
    utils::domain::front_query::QueryFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetSubscriptionProductUseCase;
#[async_trait::async_trait]
pub trait GetSubscriptionProductUseCaseTrait {
    async fn execute(
        query: QueryFront<SubscriptionProduct>,
    ) -> Result<JsonAdvanced<Vec<SubscriptionProduct>>, GetSubscriptionProductError>;
}
