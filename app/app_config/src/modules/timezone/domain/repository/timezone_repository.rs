use ac_struct_back::schemas::config::timezone::timezone::Timezone;
use surrealdb::{Surreal, engine::any::Any};

#[async_trait::async_trait]
pub trait TimezoneRepository {
    async fn insert_timezones(
        timezones: Vec<Timezone>,
        db: &Surreal<Any>,
    ) -> Result<Vec<Timezone>, surrealdb::Error>;
}
