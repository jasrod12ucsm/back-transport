use derive_more::Display;
use ntex::web::{self, WebResponseError};
use serde_json::json;

#[derive(Debug, Display)]
pub enum EncryptationError {
    #[display("Error on encrytation password")]
    Error,
}

impl WebResponseError for EncryptationError {
    fn status_code(&self) -> ntex::http::StatusCode {
        ntex::http::StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self, _: &ntex::web::HttpRequest) -> ntex::http::Response {
        match *self {
            EncryptationError::Error => web::HttpResponse::build(self.status_code())
                .set_header("content-type", "application/json; charset=utf-8")
                .json(&json!({ "error": self.to_string(), "statudCode": 404})),
        }
    }
}

