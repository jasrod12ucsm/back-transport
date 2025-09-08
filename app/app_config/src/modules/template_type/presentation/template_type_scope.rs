use ntex::web::{ServiceConfig, scope};
pub fn template_type_scope(cnf: &mut ServiceConfig) {
    cnf.service(
        scope("/")
            .service(super::template_type_controller::get_template_types)
            .service(super::template_type_controller::create_template_type)
            .service(super::template_type_controller::update_template_type)
            .service(super::template_type_controller::delete_template_type)
            .service(super::template_type_controller::get_one_template_type)
            .service(super::template_type_controller::update_template_type_by_id),
    );
}
