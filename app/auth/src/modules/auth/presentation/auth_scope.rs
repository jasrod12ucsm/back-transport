// use ntex::web::{self, ServiceConfig, middleware, scope};
//
// pub fn auth_scope(cnf: &mut ServiceConfig) {
//     cnf.service(
//         scope("/").service(super::auth_controller::login).service(
//             web::resource("/rntkn")
//                 .wrap(middleware::Logger::default()) // <--- middleware específico
//                 .route(web::get().to(super::auth_controller::rntkn)),
//         ),
//     );
// }
