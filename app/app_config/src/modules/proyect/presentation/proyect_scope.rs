use ntex::web::{ServiceConfig, scope};

use super::proyect_controller;
pub fn proyect_scope(cnf: &mut ServiceConfig) {
    cnf.service(
        scope("/")
            .service(proyect_controller::update_template_by_id)
            .service(proyect_controller::get_templates)
            .service(proyect_controller::create_template)
            .service(proyect_controller::delete_template)
            .service(proyect_controller::update_template)
            .service(proyect_controller::get_one_template),
    );
}
