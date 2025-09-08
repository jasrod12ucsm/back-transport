use std::sync::{Arc, OnceLock};

use ac_struct_back::{
    import::macro_import::Schema,
    schemas::{
        auth::{
            user::user::SchemaUserConfig,
            user_email_verify::user_email_verify::SchemaUserEmailVerify,
        },
        config::template::template::SchemaTemplate,
    },
    utils::domain::collection::{Collection, SchemaCollection},
};
use common::public::functions::auth_surreal::SurrealManager;
use config::env_config::EnvConfig;
use deadpool::{Runtime, managed::Pool};
use modules::{
    auth::presentation::auth_scope::auth_scope,
    user_management::{
        domain::{
            cases::register_use_case::{RegisterUseCase, RegisterUseCasePrivate},
            models::send_email_model::{EmailMessage, SendEmailModel},
        },
        presentation::user_management_scope::user_management_scope,
    },
};
use ntex::{
    time::Seconds,
    web::{self, HttpRequest, middleware::Logger, scope},
};
pub mod config;
pub mod modules;
pub mod utils;
use ntex_cors::Cors;
use once_cell::sync::Lazy;
use surrealdb::{
    Surreal,
    engine::{any, remote::ws::Ws},
    opt::auth::{Database, Namespace},
};
use tokio::sync::mpsc;
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

    let user_queries: Box<dyn Schema + Sync + Send> = Box::new(SchemaUserConfig);
    let user_email_config: Box<dyn Schema + Sync + Send> = Box::new(SchemaUserEmailVerify);
    let template: Box<dyn Schema + Sync + Send> = Box::new(SchemaTemplate);
    db.use_ns("ac")
        .use_db("ac")
        .await
        .expect("Failed to select namespace and database");
    let collections = vec![user_queries, user_email_config, template];
    let res = db
        .query("INFO FOR DATABASE;")
        .await
        .expect("Failed to get database info");
    println!("Database info: {:?}", res);
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

    let (tx, mut rx) = mpsc::channel::<EmailMessage>(100);

    tokio::spawn(async move {
        while let Some(email) = rx.recv().await {
            if let Err(e) = RegisterUseCase::send_email(email.email, email.code, &email.db).await {
                eprintln!("Error al enviar correo: {:?}", e);
            }
        }
    });
    //poner el sender en un arc
    let tx = Arc::new(SendEmailModel { tx });

    web::HttpServer::new(move || {
        web::App::new()
            .wrap(Logger::default())
            .wrap(
                Cors::new()
                    .allowed_origin(CONFIG.ALLOWED_ORIGIN.clone().as_str())
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH"])
                    .finish(),
            )
            .state(tx.clone())
            .service(scope("/user_management").configure(user_management_scope))
            .service(scope("/auth").configure(auth_scope))
    })
    .backlog(2048)
    .client_timeout(Seconds::new(10))
    .disconnect_timeout(Seconds(4))
    .enable_affinity()
    .headers_read_rate(Seconds(2), Seconds(10), 100)
    .keep_alive(3)
    .payload_read_rate(Seconds(1), Seconds(10), 1024)
    .workers(num_cpus::get())
    .bind((CONFIG.AUTH_IP.clone(), CONFIG.AUTH_PORT))?
    .run()
    .await
}
