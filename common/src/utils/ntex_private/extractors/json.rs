use std::time::Instant;
use std::{fmt, ops};

use futures::StreamExt;
use serde::{de::DeserializeOwned, Serialize};

use ntex::http::{Payload, Response, StatusCode};
use ntex::web::error::{ErrorRenderer, WebResponseError};
use ntex::web::{FromRequest, HttpRequest, Responder};
use serde_json::Value;
use validator::Validate;

use crate::utils::ntex_private::extractors::errors::PayloadSizes;

use super::errors::{JsonError, ValidationFieldsErrorStruct};
pub struct JsonAdvanced<T>(pub T);

impl<T> JsonAdvanced<T> {
    /// Deconstruct to an inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> ops::Deref for JsonAdvanced<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::DerefMut for JsonAdvanced<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> fmt::Debug for JsonAdvanced<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Json").field(&self.0).finish()
    }
}

impl<T> fmt::Display for JsonAdvanced<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<T: Serialize, Err: ErrorRenderer> Responder<Err> for JsonAdvanced<T>
where
    Err::Container: From<JsonError>,
{
    async fn respond_to(self, req: &HttpRequest) -> Response {
        let body = match serde_json::to_string(&self.0) {
            Ok(body) => body,
            Err(e) => return e.error_response(req),
        };

        Response::build(StatusCode::OK)
            .content_type("application/json")
            .body(body)
    }
}

/// Json extractor. Allow to extract typed information from request's
/// payload.
///
/// To extract typed information from request's body, the type `T` must
/// implement the `Deserialize` trait from *serde*.
///
/// [**JsonConfig**](struct.JsonConfig.html) allows to configure extraction
/// process.
///
/// ## Example
///
/// ```rust
/// use ntex::web;
///
/// #[derive(serde::Deserialize)]
/// struct Info {
///     username: String,
/// }
///
/// /// deserialize `Info` from request's body
/// async fn index(info: web::types::Json<Info>) -> String {
///     format!("Welcome {}!", info.username)
/// }
///
/// fn main() {
///     let app = web::App::new().service(
///         web::resource("/index.html").route(
///            web::post().to(index))
///     );
/// }
/// ```
///

#[derive(Clone)]
pub struct JsonConfigAdvanced {
    limit: usize,
}

impl JsonConfigAdvanced {
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

impl Default for JsonConfigAdvanced {
    fn default() -> Self {
        JsonConfigAdvanced { limit: 32768 }
    }
}

impl<T, Err: ErrorRenderer> FromRequest<Err> for JsonAdvanced<T>
where
    T: DeserializeOwned + 'static,
{
    //TODO create validate y modificar todos los endpoint
    type Error = JsonError;
    async fn from_request(req: &HttpRequest, payload: &mut Payload) -> Result<Self, Self::Error> {
        //los del time
        let start_time = Instant::now();
        // 1. Validación mejorada del Content-Type
        let content_type = req
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok());
        let is_json = content_type.map_or(false, |ct| {
            let ct_lower = ct.to_ascii_lowercase();
            ct_lower.starts_with("application/json") || ct_lower.starts_with("text/json")
        });

        if !is_json {
            return Err(JsonError::JsonBasicTransformError);
        }

        // 2. Lógica de límite dinámico con protección múltiple
        const DEFAULT_MAX: usize = 32_768; // 32KB
        const ABSOLUTE_MAX: usize = 10_485_760; // 10MB - Límite físico

        let requested_limit = req
            .headers()
            .get("longitude_json")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX);
        let size_limit = requested_limit.clamp(1, ABSOLUTE_MAX);
        // 3. Pre-alocación inteligente con límites
        let capacity = req
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .map(|size| size.min(size_limit)) // Usamos el límite dinámico
            .unwrap_or_else(|| DEFAULT_MAX.min(size_limit));
        //tiempo de validaciones
        let dureation_val = start_time.elapsed();
        println!("Tiempo de validaciones: {} ns", dureation_val.as_nanos());
        let mut body = Vec::with_capacity(capacity);

        // 4. Lectura del payload con límite dinámico real
        let mut current_size: usize = 0;

        while let Some(chunk) = payload.next().await {
            let chunk = chunk.map_err(|_| JsonError::JsonBasicTransformError)?;

            // Verificación acumulativa del tamaño
            current_size = current_size.saturating_add(chunk.len());
            if current_size > size_limit {
                return Err(JsonError::PayloadSizeExceeded(PayloadSizes {
                    actual_size: current_size,
                    max_size: size_limit,
                }));
            }

            body.extend_from_slice(&chunk);
        }
        println!("Tamaño del payload: {}", current_size);

        // 5. Deserialización segura con reutilización de memoria
        let mut json_data = body;

        println!(
            "{}",
            serde_json::from_slice::<Value>(&json_data)
                .map_err(|e| Value::from(e.to_string()))
                .unwrap_or_else(|e| e)
        ); // imprime: "hello"
        let initial_serialice = Instant::now();
        let data: T = simd_json::from_slice(&mut json_data)
            .map_err(|e| JsonError::JsonSerializeError(e.to_string()))?;
        let duration = initial_serialice.elapsed();
        println!("Tiempo de serialización: {} ns", duration.as_nanos());
        let time = Instant::now();
        // 6. Validación de datos con trazabilidad
        // data.validate().map_err(|err| {
        //     let error_details = ValidationFieldsErrorStruct {
        //         description: err.clone(),
        //         status_code: 400,
        //         error: err.to_string(),
        //     };
        //     JsonError::ValidationFieldsError(error_details)
        // })?;
        let duration = start_time.elapsed();
        let millis = duration.as_millis();
        let nanos = duration.as_nanos();
        let val_estructura = time.elapsed();
        println!(
            "Tiempo de validación de estructura: {} ns, {} ms",
            val_estructura.as_nanos(),
            val_estructura.as_millis()
        );
        println!("Tiempo de ejecución: {} ns, {} ms", nanos, millis);
        Ok(JsonAdvanced(data))
    }
}
