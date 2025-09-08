use std::sync::{Arc, OnceLock};

use ac_struct_back::{
    import::macro_import::Schema,
    schemas::{
        auth::subscription_product::subscription_product::SchemaSubscriptionProduct,
        config::{
            country::country::SchemaCountry, template::template::SchemaTemplate,
            template_type::template_type::SchemaTemplateType, timezone::timezone::SchemaTimezone,
        },
    },
    utils::domain::collection::{Collection, SchemaCollection},
};
use common::public::functions::auth_surreal::SurrealManager;
use config::env_config::EnvConfig;
use deadpool::{Runtime, managed::Pool};
use ntex::{
    time::Seconds,
    web::{self, middleware::Logger, scope},
};
pub mod config;
pub mod modules;
pub mod utils;

use ntex_cors::Cors;
use once_cell::sync::Lazy;
use surrealdb::{engine::any, opt::auth::Namespace};
use tonic::transport::Channel;
use utils::{domain::models::grcp_message::GrpcMessage, infrastructure::grcp_client::geocache};
const CONFIG_BYTES: &[u8] = include_bytes!("../config/config.json");
static CONFIG: Lazy<EnvConfig> =
    Lazy::new(|| serde_json::from_slice(CONFIG_BYTES).expect("JSON inválido"));

static SURREAL_POOL: OnceLock<Pool<SurrealManager>> = OnceLock::new();
pub fn try_get_surreal_pool() -> Option<&'static Pool<SurrealManager>> {
    SURREAL_POOL.get()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    println!("numcpus: {}", num_cpus::get());

    let endpoint = Channel::from_static("hhtps://[::1]:50051").connect_lazy();
    let grcp_message = Arc::new(GrpcMessage::new(endpoint));
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
        CONFIG.API_IP, CONFIG.API_PORT
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
            .state(grcp_message.clone())
            .service(scope("/geo").configure(modules::geo::presentation::geo_scope::geo_scope))
    })
    .backlog(2048)
    .client_timeout(Seconds::new(10))
    .disconnect_timeout(Seconds(4))
    .enable_affinity()
    .headers_read_rate(Seconds(2), Seconds(10), 100)
    .keep_alive(3)
    .payload_read_rate(Seconds(1), Seconds(10), 1024)
    .workers(num_cpus::get())
    .bind((CONFIG.API_IP.clone(), CONFIG.API_PORT))?
    .run()
    .await
}
