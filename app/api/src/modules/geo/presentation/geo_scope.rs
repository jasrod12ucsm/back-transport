use ntex::web::{ServiceConfig, scope};

use super::geo_controller;
pub fn geo_scope(cnf: &mut ServiceConfig) {
    cnf.service(scope("/").service(geo_controller::get_ids_in_polygon));
}
