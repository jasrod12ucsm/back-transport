use deadpool::managed::{Manager, Metrics, RecycleResult};
use std::{future::Future, sync::Arc, time::Duration};
use surrealdb::{
    engine::any::{self, Any},
    opt::auth::Namespace,
    Error, Surreal,
};
use tokio::time::{self, Instant};
#[derive(Debug)]
pub struct SurrealManager {
    pub db_url: String,
    pub username: String,
    pub password: String,
    pub ns: String,
    pub db: String,
}
#[derive(Debug, Clone)]
pub struct AuthenticatedSurreal {
    pub client: Surreal<Any>,
    pub last_auth: Instant,
    pub credentials: Arc<SurrealManager>,
}

impl AuthenticatedSurreal {
    pub async fn ensure_authenticated(&mut self) -> Result<(), Error> {
        if self.last_auth.elapsed() <= Duration::from_secs(50 * 60) {
            return Ok(());
        }

        let token = self
            .client
            .signin(Namespace {
                username: &self.credentials.username,
                password: &self.credentials.password,
                namespace: &self.credentials.ns,
            })
            .await?;

        self.client
            .use_ns(&self.credentials.ns)
            .use_db(&self.credentials.db)
            .await?;

        self.client.authenticate(token).await?;
        self.last_auth = Instant::now();
        Ok(())
    }
}

impl Manager for SurrealManager {
    type Type = AuthenticatedSurreal;
    type Error = Error;

    fn create(&self) -> impl Future<Output = Result<Self::Type, Self::Error>> + Send {
        let db_url = self.db_url.clone();
        let username = self.username.clone();
        let password = self.password.clone();
        let ns = self.ns.clone();
        let db = self.db.clone();

        async move {
            let credentials = Arc::new(SurrealManager {
                db_url: db_url.clone(),
                username: username.clone(),
                password: password.clone(),
                ns: ns.clone(),
                db: db.clone(),
            });

            let client = any::connect(&db_url).await?;
            client
                .signin(Namespace {
                    username: &username,
                    password: &password,
                    namespace: &ns,
                })
                .await?;

            client.use_ns(&ns).use_db(&db).await?;

            Ok(AuthenticatedSurreal {
                client,
                last_auth: Instant::now(),
                credentials,
            })
        }
    }

    fn recycle(
        &self,
        obj: &mut Self::Type,
        _metrics: &Metrics,
    ) -> impl Future<Output = RecycleResult<Self::Error>> + Send {
        async move {
            let mut attempts = 0;
            let max_attempts = 3;
            loop {
                match obj.ensure_authenticated().await {
                    Ok(()) => return Ok(()),
                    Err(_) if attempts < max_attempts => {
                        attempts += 1;
                        time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
}
