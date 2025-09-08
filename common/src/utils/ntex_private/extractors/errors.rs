use ntex::{
    http,
    web::{self, error::QueryPayloadError},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BaseError {
    pub error: String,
    pub message: String,
    #[serde(rename = "statusCode")]
    pub status_code: i32,
}

impl BaseError {
    pub fn new(error: String, message: String, status_code: i32) -> Self {
        Self {
            error,
            message,
            status_code,
        }
    }
}


//multiparte
#[derive(Debug, derive_more::Display)]
pub enum MultipartError {
    #[display(fmt = "Validation error on field:")]
    ValidationError(ValidationErrorStruct),
    #[display(fmt = "Error reading multipart data")]
    FileChargeError,
    #[display(fmt = "Error reading multipart data")]
    ValidationFieldsError(ValidationFieldsErrorStruct),
}

//json
#[derive(Debug, derive_more::Display)]
pub enum JsonError {
    #[display(fmt = "Serialize error")]
    JsonSerializeError(String),
    #[display(fmt = "Error reading json data")]
    ValidationFieldsError(ValidationFieldsErrorStruct),
    #[display(fmt = "Error reading json data")]
    InternalServerError,
    #[display(fmt = "transform payload error")]
    JsonBasicTransformError,
    #[display(fmt = "Payload size exceeded")]
    PayloadSizeExceeded(PayloadSizes),
}
#[derive(Debug, Serialize)]
pub struct PayloadSizes {
    pub max_size: usize,
    pub actual_size: usize,
}

#[derive(Debug, Serialize)]
pub struct ValidationFieldsErrorStruct {
    pub error: String,
    pub description: validator::ValidationErrors,
    #[serde(rename = "statusCode")]
    pub status_code: u16,
}

impl ValidationFieldsErrorStruct {
    pub fn new(description: validator::ValidationErrors) -> Self {
        Self {
            error: description.to_string(),
            description,
            status_code: 400,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ValidationErrorStruct {
    pub error: String,

    pub field: Vec<String>,
    #[serde(rename = "statusCode")]
    pub status_code: u16,
}

impl ValidationErrorStruct {
    pub fn new(field: Vec<String>) -> Self {
        Self {
            error: "error en la validacion".to_string(),
            field,
            status_code: 400,
        }
    }
}

impl From<&ValidationErrorStruct> for BaseError {
    fn from(value: &ValidationErrorStruct) -> Self {
        let error = value.error.clone();
        Self {
            error: value.error.to_owned(),
            message: error,
            status_code: value.status_code as i32,
        }
    }
}

impl From<&ValidationFieldsErrorStruct> for BaseError {
    fn from(value: &ValidationFieldsErrorStruct) -> Self {
        let error = value.error.clone();
        Self {
            error: value.error.to_owned(),
            message: error,
            status_code: value.status_code as i32,
        }
    }
}

impl web::error::WebResponseError for MultipartError {
    fn error_response(&self, _: &web::HttpRequest) -> web::HttpResponse {
        match *self {
            MultipartError::ValidationError(ref field) => {
                let error: BaseError = field.into();
                web::HttpResponse::build(self.status_code())
                    .set_header("content-type", "text/json; charset=utf-8")
                    .json(&error)
            }
            MultipartError::FileChargeError => {
                let error = BaseError {
                    error: "file charge error".to_string(),
                    message: self.to_string(),
                    status_code: self.status_code().as_u16() as i32,
                };
                web::HttpResponse::build(self.status_code())
                    .set_header("content-type", "text/json; charset=utf-8")
                    .json(&error)
            }
            MultipartError::ValidationFieldsError(ref field) => {
                let error: BaseError = field.into();
                web::HttpResponse::build(self.status_code())
                    .set_header("content-type", "text/json; charset=utf-8")
                    .json(&error)
            }
        }
    }

    fn status_code(&self) -> http::StatusCode {
        match *self {
            MultipartError::ValidationError { .. } => http::StatusCode::BAD_REQUEST,
            MultipartError::FileChargeError => http::StatusCode::FORBIDDEN,
            MultipartError::ValidationFieldsError { .. } => http::StatusCode::BAD_REQUEST,
        }
    }
}

impl web::error::WebResponseError for JsonError {
    fn status_code(&self) -> http::StatusCode {
        match *self {
            JsonError::InternalServerError => http::StatusCode::INTERNAL_SERVER_ERROR,
            JsonError::JsonSerializeError(_) => http::StatusCode::BAD_REQUEST,
            JsonError::ValidationFieldsError(..) => http::StatusCode::BAD_REQUEST,
            JsonError::JsonBasicTransformError => http::StatusCode::BAD_REQUEST,
            JsonError::PayloadSizeExceeded(_) => http::StatusCode::PAYLOAD_TOO_LARGE,
        }
    }

    fn error_response(&self, _: &web::HttpRequest) -> web::HttpResponse {
        match self {
            JsonError::PayloadSizeExceeded(size) => {
                let error = BaseError {
                    error: "payload size exceeded".to_string(),
                    message: format!(
                        "Payload size exceeded (max,act): {},{}",
                        size.max_size, size.actual_size
                    ),
                    status_code: self.status_code().as_u16() as i32,
                };
                web::HttpResponse::build(self.status_code())
                    .set_header("content-type", "text/json; charset=utf-8")
                    .json(&error)
            }

            JsonError::InternalServerError => {
                let error = BaseError {
                    error: "internal server error".to_string(),
                    message: self.to_string(),
                    status_code: self.status_code().as_u16() as i32,
                };
                web::HttpResponse::build(self.status_code())
                    .set_header("content-type", "text/json; charset=utf-8")
                    .json(&error)
            }
            JsonError::JsonSerializeError(value) => {
                let error = BaseError {
                    error: "serialize error".to_string(),
                    message: value.to_string(),
                    status_code: self.status_code().as_u16() as i32,
                };
                web::HttpResponse::build(self.status_code())
                    .set_header("content-type", "text/json; charset=utf-8")
                    .json(&error)
            }
            JsonError::ValidationFieldsError(ref field) => {
                let error: BaseError = field.into();
                web::HttpResponse::build(self.status_code())
                    .set_header("content-type", "text/json; charset=utf-8")
                    .json(&error)
            }
            JsonError::JsonBasicTransformError => {
                let error = BaseError {
                    error: "transform payload error".to_string(),
                    message: self.to_string(),
                    status_code: self.status_code().as_u16() as i32,
                };
                web::HttpResponse::build(self.status_code())
                    .set_header("content-type", "text/json; charset=utf-8")
                    .json(&error)
            }
        }
    }
}

#[derive(Debug, derive_more::Display)]
pub struct QueryAdvancedError {
    pub query: QueryPayloadError,
}

impl web::error::WebResponseError for QueryAdvancedError {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::BAD_REQUEST
    }

    fn error_response(&self, _: &web::HttpRequest) -> web::HttpResponse {
        let error = BaseError {
            error: "query error".to_string(),
            message: self.query.to_string(),
            status_code: self.status_code().as_u16() as i32,
        };
        web::HttpResponse::build(self.status_code())
            .set_header("content-type", "text/json; charset=utf-8")
            .json(&error)
    }
}
