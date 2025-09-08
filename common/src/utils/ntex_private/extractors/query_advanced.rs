use std::{fmt, ops};

use ntex::{
    http::Payload,
    web::{error::QueryPayloadError, ErrorRenderer, FromRequest, HttpRequest},
};
use serde::de;

use super::errors::QueryAdvancedError;
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct QueryAdvanced<T>(pub T);

impl<T> QueryAdvanced<T> {
    /// Deconstruct to a inner value
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Get query parameters from the path
    pub fn from_query(query_str: &str) -> Result<Self, QueryAdvancedError>
    where
        T: de::DeserializeOwned,
    {
        serde_urlencoded::from_str::<T>(query_str)
            .map(|val| Ok(QueryAdvanced(val)))
            .unwrap_or_else(move |e| {
                Err(QueryAdvancedError {
                    query: QueryPayloadError::Deserialize(e),
                })
            })
    }
}

impl<T> ops::Deref for QueryAdvanced<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::DerefMut for QueryAdvanced<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for QueryAdvanced<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for QueryAdvanced<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Extract typed information from the request's query.
///
/// ## Example
///
/// ```rust
/// use ntex::web;
///
/// #[derive(Debug, serde::Deserialize)]
/// pub enum ResponseType {
///    Token,
///    Code
/// }
///
/// #[derive(serde::Deserialize)]
/// pub struct AuthRequest {
///    id: u64,
///    response_type: ResponseType,
/// }
///
/// // Use `Query` extractor for query information.
/// // This handler get called only if request's query contains `username` field
/// // The correct request for this handler would be `/index.html?id=64&response_type=Code"`
/// async fn index(info: web::types::Query<AuthRequest>) -> String {
///     format!("Authorization request for client with id={} and type={:?}!", info.id, info.response_type)
/// }
///
/// fn main() {
///     let app = web::App::new().service(
///        web::resource("/index.html")
///            .route(web::get().to(index))); // <- use `Query` extractor
/// }
/// ```
impl<T, Err> FromRequest<Err> for QueryAdvanced<T>
where
    T: de::DeserializeOwned,
    Err: ErrorRenderer,
{
    type Error = QueryAdvancedError;

    #[inline]
    async fn from_request(req: &HttpRequest, _: &mut Payload) -> Result<Self, Self::Error> {
        serde_urlencoded::from_str::<T>(req.query_string())
            .map(|val| Ok(QueryAdvanced(val)))
            .unwrap_or_else(move |e| {
                let e = QueryAdvancedError {
                    query: QueryPayloadError::Deserialize(e),
                };

                log::debug!(
                    "Failed during Query extractor deserialization. \
                     Request path: {:?}",
                    req.path()
                );
                Err(e)
            })
    }
}
