use ntex::web::{ServiceConfig, scope};
pub fn timezone_scope(cnf: &mut ServiceConfig) {
    cnf.service(scope("/").service(super::timezone_controller::create_timezone));
}
