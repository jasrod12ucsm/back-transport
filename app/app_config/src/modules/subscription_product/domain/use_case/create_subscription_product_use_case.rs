use ac_struct_back::schemas::auth::subscription_product::subscription_product::{
    CreateSubscriptionProductError, SubscriptionProduct,
    subscriptionproductdtosubscriptionproduct::SubscriptionProductDto,
};

pub struct CreateSubscriptionProductUseCase;
#[async_trait::async_trait]
pub trait CreateProductUseCaseTrait {
    async fn create_product(
        product: SubscriptionProductDto,
    ) -> Result<SubscriptionProduct, CreateSubscriptionProductError>;
}
