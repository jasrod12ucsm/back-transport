use ntex::web::{ServiceConfig, scope};
pub fn user_management_scope(cnf: &mut ServiceConfig) {
    cnf.service(
        scope("/")
            .service(super::user_management_controller::register_user)
            .service(super::user_management_controller::generate_code)
            .service(super::user_management_controller::verify_code),
    );
}
