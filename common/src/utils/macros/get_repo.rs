#[macro_export]
macro_rules! get_repositories {
    ($repo:expr, $err_ty:path, $($repo_name:ident => $var_name:ident),*) => {
        {
            $(
                let $var_name = $repo.get_repository::<$repo_name>().await
                    .map_err(|_| $err_ty("internal server error"))?;
            )*
            Ok(($($var_name),*))
        }
    };
}



// macros.rs

// macros.rs

#[macro_export]
macro_rules! define_repository {
    ($repo_name:ident, $model:ty, $model_with_id:ty, $repo_static:ident) => {
        use once_cell::sync::OnceCell;
        use common::utils::ntex_private::repository::public_repository::{
            PublicRepository, Repository, SetPublicRepository,
        };
        use mongodb::{Client, Collection};
        use bod_models::shared::schema::BaseColleccionNames;
        use std::sync::Arc;

        static $repo_static: OnceCell<$repo_name> = OnceCell::new();

        #[derive(Clone)]
        pub struct $repo_name {
            collection: Collection<$model>,
            collection_id: Collection<$model_with_id>,
            client: Arc<Client>,
        }

        impl Repository<$model, $model_with_id> for $repo_name {
            fn get_collection(&self) -> &Collection<$model> {
                &self.collection
            }

            fn get_client(&self) -> &Client {
                &self.client
            }
            
            fn get_collection_for_id(&self) -> &Collection<$model_with_id> {
                &self.collection_id
            }
        }

        impl $repo_name {
            pub async fn new(repository: &PublicRepository) -> Result<(), mongodb::error::Error> {
                let client = repository.get_client();
                let db_name = <$model>::get_database_name();
                let coll_name = <$model>::get_collection_name();
                
                let repo = Self {
                    collection: client.database(&db_name).collection(&coll_name),
                    collection_id: client.database(&db_name).collection(&coll_name),
                    client: client,
                };
                
                $repo_static.set(repo)
                    .map_err(|_| mongodb::error::Error::custom("Repository already initialized"))
            }

            pub fn get() -> &'static Self {
                $repo_static.get().expect("Repository no inicializado")
            }
        }

        #[async_trait::async_trait]
        impl SetPublicRepository for $repo_name {
            type RepositoryType = $repo_name;

            async fn set_repository(
                repository: &PublicRepository,
            ) -> Result<Self::RepositoryType, mongodb::error::Error> {
                if $repo_static.get().is_none() {
                    Self::new(repository).await?;
                }
                Ok(Self::get().clone())
            }
        }
    };
}