use async_trait::async_trait;
use std::{future::Future, pin::Pin};

#[async_trait]
pub trait AsyncMapErr<T, E, F, R>
where
    F: FnOnce(E) -> Pin<Box<dyn Future<Output = R> + Send>> + Send,
{
    async fn async_map_err(self, f: F) -> Result<T, R>;
}

#[async_trait]
impl<T, E, F, R> AsyncMapErr<T, E, F, R> for Result<T, E>
where
    F: 'static + FnOnce(E) -> Pin<Box<dyn Future<Output = R> + Send>> + Send,
    T: Send,
    E: Send,
    R: Send,
{
    async fn async_map_err(self, f: F) -> Result<T, R> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(f(e).await),
        }
    }
}
