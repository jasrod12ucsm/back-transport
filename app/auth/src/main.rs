use std::sync::{Arc, OnceLock};

use config::env_config::EnvConfig;
use deadpool::{Runtime, managed::Pool};

use ntex::{
    time::Seconds,
    web::{self, HttpRequest, middleware::Logger, scope},
};
pub mod config;
pub mod modules;
pub mod utils;
use ntex_cors::Cors;
use once_cell::sync::Lazy;

use tokio::sync::mpsc;
// static CONFIG: Lazy<EnvConfig> =
//     Lazy::new(|| serde_json::from_slice(CONFIG_BYTES).expect("JSON inválido"));
//
// static SURREAL_POOL: OnceLock<Pool<SurrealManager>> = OnceLock::new();
// pub fn try_get_surreal_pool() -> Option<&'static Pool<SurrealManager>> {
//     SURREAL_POOL.get()
// }
//
const CONFIG_BYTES: &[u8] = include_bytes!("../config/config.json");
static CONFIG: Lazy<EnvConfig> =
    Lazy::new(|| serde_json::from_slice(CONFIG_BYTES).expect("JSON inválido"));
#[web::get("/")]
async fn hello() -> impl web::Responder {
    web::HttpResponse::Ok().body("Hello world!")
}

#[web::post("/echo")]
async fn echo(req_body: String) -> impl web::Responder {
    web::HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl web::Responder {
    web::HttpResponse::Ok().body("Hey there!")
}
#[ntex::main]
async fn main() -> std::io::Result<()> {
    web::HttpServer::new(|| {
        web::App::new()
            .service(hello)
            .service(echo)
            .route("/hey", web::get().to(manual_hello))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
