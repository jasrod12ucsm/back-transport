use ac_struct_back::{
    schemas::auth::subscription_product::subscription_product::{
        CreateSubscriptionProductError, DeleteSubscriptionProductError,
        GetOneSubscriptionProductError, GetSubscriptionProductError, SubscriptionProduct,
        UpdateSubscriptionProductError,
        subscriptionproductdtosubscriptionproduct::SubscriptionProductDto,
        updatesubscriptionproductdtosubscriptionproduct::UpdateSubscriptionProductDto,
    },
    utils::domain::front_query::{QueryFront, UpdateRequestBuilderFront},
};
use common::utils::ntex_private::extractors::{json::JsonAdvanced, query_advanced::QueryAdvanced};
use ntex::web;

use crate::modules::subscription_product::domain::use_case::{
    create_subscription_product_use_case::{
        CreateProductUseCaseTrait, CreateSubscriptionProductUseCase,
    },
    delete_subscription_product_use_case::{
        DeleteSubscriptionProductUseCase, DeleteSubscriptionProductUseCasePublic,
    },
    get_one_subscription_prodcut_use_case::{
        GetOneSubscriptionProductUseCase, GetOneSubscriptionProductUseCaseTrait,
    },
    get_subscription_product_use_case::{
        GetSubscriptionProductUseCase, GetSubscriptionProductUseCaseTrait,
    },
    update_subscription_product_use_case::{
        UpdateSubscriptionProductUseCase, UpdateSubscriptionProductUseCaseTrait,
    },
};

#[web::post("/")]
pub async fn create_subscription_product(
    dto: JsonAdvanced<SubscriptionProductDto>,
) -> Result<JsonAdvanced<SubscriptionProduct>, CreateSubscriptionProductError> {
    CreateSubscriptionProductUseCase::create_product(dto.0)
        .await
        .map(JsonAdvanced)
}

#[web::get("/{id}")]
pub async fn subscription_product_id(
    query: QueryAdvanced<QueryFront<SubscriptionProduct>>,
    id: web::types::Path<String>,
) -> Result<JsonAdvanced<Option<SubscriptionProduct>>, GetOneSubscriptionProductError> {
    GetOneSubscriptionProductUseCase::execute(query.0, &id).await
}

//get all
#[web::get("/")]
pub async fn subscription_product(
    query: QueryAdvanced<QueryFront<SubscriptionProduct>>,
) -> Result<JsonAdvanced<Vec<SubscriptionProduct>>, GetSubscriptionProductError> {
    GetSubscriptionProductUseCase::execute(query.0).await
}

//delete
#[web::delete("/{id}")]
pub async fn delete_subscription_product(
    id: web::types::Path<String>,
) -> Result<JsonAdvanced<SubscriptionProduct>, DeleteSubscriptionProductError> {
    let id = id.into_inner();
    DeleteSubscriptionProductUseCase::execute(&id).await
}

//update one
#[web::patch("/{id}")]
pub async fn update_subscription_product(
    dto: JsonAdvanced<UpdateSubscriptionProductDto>,
    id: web::types::Path<String>,
    query: QueryAdvanced<UpdateRequestBuilderFront<SubscriptionProduct>>,
) -> Result<JsonAdvanced<Vec<SubscriptionProduct>>, UpdateSubscriptionProductError> {
    let id = id.into_inner();
    UpdateSubscriptionProductUseCase::execute(Some(id), query.0, dto).await
}

//update all
//
#[web::patch("/")]
pub async fn update_subscription_product_all(
    dto: JsonAdvanced<UpdateSubscriptionProductDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<SubscriptionProduct>>,
) -> Result<JsonAdvanced<Vec<SubscriptionProduct>>, UpdateSubscriptionProductError> {
    UpdateSubscriptionProductUseCase::execute(None, query.0, dto).await
}
