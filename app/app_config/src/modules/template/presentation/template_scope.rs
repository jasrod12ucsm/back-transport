use ntex::web::{ServiceConfig, scope};

pub fn template_scope(cnf: &mut ServiceConfig) {
    cnf.service(
        scope("/")
            .service(super::template_controller::create_template)
            .service(super::template_controller::update_template)
            .service(super::template_controller::get_templates)
            .service(super::template_controller::get_one_template)
            .service(super::template_controller::delete_template)
            .service(super::template_controller::update_template_by_id),
    );
}
