use std::sync::{Arc, OnceLock};

use ac_struct_back::{
    import::macro_import::Schema,
    schemas::{
        auth::{
            subscription_product::subscription_product::SchemaSubscriptionProduct,
            user::user::SchemaUserConfig,
            user_email_verify::user_email_verify::SchemaUserEmailVerify,
        },
        config::{
            country::country::SchemaCountry,
            template::template::SchemaTemplate,
            template_type::template_type::{SchemaTemplateType, TemplateType},
            timezone::timezone::SchemaTimezone,
        },
    },
    utils::domain::{
        collection::{Collection, SchemaCollection},
        query::{Query, UpdateRequestBuilder, UpdateTarget},
    },
};
use common::public::functions::auth_surreal::SurrealManager;
use config::env_config::EnvConfig;
use deadpool::{Runtime, managed::Pool};
use ntex::{
    time::Seconds,
    web::{self, HttpRequest, middleware::Logger, scope},
};
pub mod config;
pub mod modules;
pub mod utils;
#[macro_use]
extern crate lazy_static;

use ntex_cors::Cors;
use once_cell::sync::Lazy;
use surrealdb::{
    Surreal,
    engine::{any, remote::ws::Ws},
    opt::auth::{Database, Namespace},
};
use tokio::sync::mpsc;
const CONFIG_BYTES: &[u8] = include_bytes!("../config/config.json");
const PYTHON_ARCHIVE: &str = include_str!("../config/main.py");
static CONFIG: Lazy<EnvConfig> =
    Lazy::new(|| serde_json::from_slice(CONFIG_BYTES).expect("JSON inválido"));

static SURREAL_POOL: OnceLock<Pool<SurrealManager>> = OnceLock::new();
pub fn try_get_surreal_pool() -> Option<&'static Pool<SurrealManager>> {
    SURREAL_POOL.get()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    println!("numcpus: {}", num_cpus::get());
    unsafe { std::env::set_var("RUST_LOG", "ntex::middleware::logger=info") };

    env_logger::init();
    let db = any::connect(CONFIG.SURREAL_URL.as_str())
        .await
        .expect("Failed to connect to SurrealDB");

    // Select namespace and database

    let log = db
        .signin(Namespace {
            namespace: "ac",
            password: &CONFIG.SUR_PASSWORD,
            username: &CONFIG.SUR_USERNAME,
        })
        .await
        .expect("Failed to sign in to SurrealDB");
    db.authenticate(log)
        .await
        .expect("Failed to authenticate SurrealDB connection");

    println!(
        "Starting server on http://{}:{}",
        CONFIG.AUTH_IP, CONFIG.AUTH_PORT
    );

    let email_type: Box<dyn Schema + Sync + Send> = Box::new(SchemaTemplateType);
    let template: Box<dyn Schema + Sync + Send> = Box::new(SchemaTemplate);
    let subscription_product: Box<dyn Schema + Sync + Send> = Box::new(SchemaSubscriptionProduct);
    let country: Box<dyn Schema + Sync + Send> = Box::new(SchemaCountry);
    let timezone: Box<dyn Schema + Sync + Send> = Box::new(SchemaTimezone);
    db.use_ns("ac")
        .use_db("ac")
        .await
        .expect("Failed to select namespace and database");
    let res = db
        .query("INFO FOR DATABASE;")
        .await
        .expect("Failed to get database info");
    println!("Database info: {:?}", res);
    let collections = vec![
        email_type,
        template,
        subscription_product,
        country,
        timezone,
    ];
    let _ = Collection::new(collections)
        .define_all(&db)
        .await
        .expect("Failed to define schemas");
    let manager = SurrealManager {
        db_url: CONFIG.SURREAL_URL.clone(),
        username: CONFIG.SUR_USERNAME.clone(),
        password: CONFIG.SUR_PASSWORD.clone(),
        ns: "ac".to_string(),
        db: "ac".to_string(),
    };

    let pool = Pool::builder(manager)
        .wait_timeout(Some(Seconds::new(10).into()))
        .max_size(300)
        .runtime(Runtime::Tokio1)
        .build()
        .expect("Failed to create SurrealDB pool");

    SURREAL_POOL.set(pool).expect("Pool already initialized");

    web::HttpServer::new(move || {
        web::App::new()
            .wrap(Logger::default())
            .wrap(
                Cors::new()
                    .allowed_origin(CONFIG.ALLOWED_ORIGIN.clone().as_str())
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH"])
                    .finish(),
            )
            .service(
                scope("/proyect")
                    .configure(modules::proyect::presentation::proyect_scope::proyect_scope),
            )
            .service(
                scope("/app_charge").configure(
                    modules::app_charge::presentation::app_charge_scope::app_charge_scope,
                ),
            )
    })
    .backlog(2048)
    .client_timeout(Seconds::new(10000))
    .disconnect_timeout(Seconds(10000))
    .enable_affinity()
    .keep_alive(1000)
    .workers(num_cpus::get())
    .bind((CONFIG.AUTH_IP.clone(), CONFIG.AUTH_PORT))?
    .run()
    .await
}
