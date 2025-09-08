use std::borrow::Cow;

use ac_struct_back::{
    import::macro_import::TableName,
    schemas::auth::subscription_product::subscription_product::{
        CreateSubscriptionProductError, SubscriptionProduct,
        subscriptionproductdtosubscriptionproduct::SubscriptionProductDto,
    },
    utils::domain::query::{
        Expression, OneOrMany, Operator, Query, comparison, execute_select_query,
    },
};
use ntex::http::StatusCode;
use serde_json::Value;
use surrealdb::{Surreal, engine::any::Any};

use crate::{
    modules::subscription_product::domain::{
        models::subscription_product_id::SubscriptionProductId,
        use_case::create_subscription_product_use_case::{
            CreateProductUseCaseTrait, CreateSubscriptionProductUseCase,
        },
    },
    try_get_surreal_pool,
};
#[async_trait::async_trait]
impl CreateProductUseCaseTrait for CreateSubscriptionProductUseCase {
    async fn create_product(
        product: SubscriptionProductDto,
    ) -> Result<SubscriptionProduct, CreateSubscriptionProductError> {
        let db = try_get_surreal_pool()
            .ok_or_else(|| CreateSubscriptionProductError {
                message: format!("Could not get surreal pool"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| CreateSubscriptionProductError {
                message: format!("Could not get surreal pool"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        //verifica existencia
        Self::verify_existance(product.name.as_str(), &db.client).await?;
        //crea el SubscriptionProductDto
        Self::create_product(product, &db.client).await
    }
}

impl CreateSubscriptionProductUseCase {
    async fn create_product(
        prod: SubscriptionProductDto,
        db: &Surreal<Any>,
    ) -> Result<SubscriptionProduct, CreateSubscriptionProductError> {
        let product = SubscriptionProduct {
            name: prod.name,
            description: prod.description,
            ..Default::default()
        };
        let val: Option<SubscriptionProduct> = db
            .create(SubscriptionProduct::table_name())
            .content(product)
            .await
            .map_err(|_| CreateSubscriptionProductError {
                message: format!("Could not create product"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if let Some(val) = val {
            return Ok(val);
        } else {
            return Err(CreateSubscriptionProductError {
                message: format!("Could not create product"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }
    async fn verify_existance(
        name: &str,
        db: &Surreal<Any>,
    ) -> Result<(), CreateSubscriptionProductError> {
        let exist: OneOrMany<SubscriptionProductId> = execute_select_query(
            Query::<SubscriptionProduct>::new()
                .from(None, true)
                .condition(comparison(
                    Expression::Field(Cow::Borrowed("name")),
                    Operator::Eq,
                    Expression::Value(Value::from("name")),
                ))
                .parameter("name", Value::from(name))
                .fields(&["id"])
                .get_owned(),
            db,
            true,
        )
        .await?;
        match exist {
            OneOrMany::One(val) => {
                if let Some(id) = val {
                    return Err(CreateSubscriptionProductError {
                        message: format!("Product with name already exist"),
                        status_code: StatusCode::CONFLICT,
                    });
                } else {
                    Ok(())
                }
            }
            OneOrMany::Many(_) => Err(CreateSubscriptionProductError {
                message: format!("Product with name already exist"),
                status_code: StatusCode::CONFLICT,
            }),
        }
    }
}
