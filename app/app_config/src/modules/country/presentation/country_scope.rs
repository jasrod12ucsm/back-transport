use ntex::web::{ServiceConfig, scope};
pub fn country_scope(cnf: &mut ServiceConfig) {
    cnf.service(
        scope("/")
            .service(super::country_controller::create_countries_by_doc)
            .service(super::country_controller::get_countries)
            .service(super::country_controller::get_one_country)
            .service(super::country_controller::update_country)
            .service(super::country_controller::update_country_by_id),
    );
}
