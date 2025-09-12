use ac_struct_back::{
    common::model::base_error::BaseError, define_error_enum,
    utils::domain::errors::GenerateResponseByMessage,
};

#[derive(Debug)]
pub enum CsvError {
    InvalidFileName,
    InvalidFileContent,
    InvalidFileType,
    FileChargeError,
    DbError(String),
}
impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFileName => write!(f, stringify!(InvalidFileName)),
            Self::InvalidFileContent => write!(f, stringify!(InvalidFileContent)),
            Self::InvalidFileType => write!(f, stringify!(InvalidFileType)),
            Self::FileChargeError => write!(f, stringify!(FileChargeError)),
            Self::DbError(msg) => write!(f, "{}", msg),
        }
    }
}
impl ntex::web::error::WebResponseError for CsvError {
    fn status_code(&self) -> ntex::http::StatusCode {
        ntex::http::StatusCode::BAD_REQUEST
    }
    fn error_response(&self, _: &ntex::web::HttpRequest) -> ntex::web::HttpResponse {
        let error_type = stringify!(CsvError);
        let message = self.to_string();
        let error = BaseError::new(
            error_type,
            &message,
            ntex::http::StatusCode::BAD_REQUEST.as_u16(),
        );
        ntex::web::HttpResponse::BadRequest()
            .set_header("Content-type", "application/json; charset=utf-8")
            .json(&error)
    }
}
impl GenerateResponseByMessage for CsvError {
    fn by_message(message: String) -> Self {
        match message.as_str() {
            stringify!(InvalidFileName) => Self::InvalidFileName,
            stringify!(InvalidFileContent) => Self::InvalidFileContent,
            stringify!(InvalidFileType) => Self::InvalidFileType,
            stringify!(FileChargeError) => Self::FileChargeError,
            _ => Self::DbError(message),
        }
    }
}
