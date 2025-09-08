use ntex::web::{ServiceConfig, scope};
pub fn subscription_product(cnf: &mut ServiceConfig) {
    cnf.service(
        scope("/")
            .service(super::subscription_product_controller::create_subscription_product)
            .service(super::subscription_product_controller::subscription_product_id)
            .service(super::subscription_product_controller::subscription_product)
            .service(super::subscription_product_controller::delete_subscription_product)
            .service(super::subscription_product_controller::update_subscription_product)
            .service(super::subscription_product_controller::update_subscription_product_all),
    );
}
