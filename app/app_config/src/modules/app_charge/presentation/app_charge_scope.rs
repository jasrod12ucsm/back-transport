use super::app_charge_controller;
use ntex::web::{ServiceConfig, scope};

pub fn app_charge_scope(cnf: &mut ServiceConfig) {
    cnf.service(
        scope("/")
            .service(app_charge_controller::get_columns)
            .service(app_charge_controller::charge_file)
            .service(app_charge_controller::get_all_data)
            .service(app_charge_controller::get_scat)
            .service(app_charge_controller::charge_field_scatt)
            .service(app_charge_controller::charge_categoricals)
            .service(app_charge_controller::charge_continous),
    );
}
