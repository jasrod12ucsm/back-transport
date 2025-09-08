use ac_struct_back::{
    schemas::auth::subscription_product::subscription_product::{
        GetOneSubscriptionProductError, SubscriptionProduct,
    },
    utils::domain::front_query::QueryFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetOneSubscriptionProductUseCase;
#[async_trait::async_trait]
pub trait GetOneSubscriptionProductUseCaseTrait {
    async fn execute(
        query: QueryFront<SubscriptionProduct>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<SubscriptionProduct>>, GetOneSubscriptionProductError>;
}
