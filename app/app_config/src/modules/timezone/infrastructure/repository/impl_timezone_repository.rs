use ac_struct_back::{
    import::macro_import::TableName, schemas::config::timezone::timezone::Timezone,
};
use surrealdb::{Surreal, engine::any::Any};

use crate::modules::timezone::domain::repository::timezone_repository::TimezoneRepository;
#[async_trait::async_trait]
impl TimezoneRepository for Timezone {
    async fn insert_timezones(
        timezones: Vec<Timezone>,
        db: &Surreal<Any>,
    ) -> Result<Vec<Timezone>, surrealdb::Error> {
        let insert_action = db.insert(Timezone::table_name()).content(timezones).await?;
        Ok(insert_action.into())
    }
}
