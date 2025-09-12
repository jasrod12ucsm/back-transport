use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::Write,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::utils::traits::hashmap::HashMapToStruct;
use futures::{StreamExt, TryStreamExt};
use ntex::{
    http::{header::CONTENT_DISPOSITION, Payload},
    util::BytesMut,
    web::{ErrorRenderer, FromRequest, HttpRequest},
};
use ntex_multipart::{Field, Multipart};
use reqwest::header::CONTENT_LENGTH;
use serde::de::DeserializeOwned;
use tokio::{sync::Mutex, time};
use validator::Validate;

use super::errors::{MultipartError, ValidationErrorStruct};

#[derive(Debug)]
pub struct MultipartData<T> {
    files: Option<Vec<PreLoadFile>>,
    data: Option<T>,
}

#[derive(Debug)]
pub struct PreLoadFile {
    pub file_name: String,
    pub file_data: BytesMut,
    pub extension: String,
    pub content_type: String,
}
impl Clone for PreLoadFile {
    fn clone(&self) -> Self {
        Self {
            file_name: self.file_name.clone(),
            file_data: self.file_data.clone(),
            extension: self.extension.clone(),
            content_type: self.content_type.clone(),
        }
    }
}
impl<T: DeserializeOwned + Default> MultipartData<T> {
    //crealo, recibe la refeencia de un multipart
    pub async fn new(mut payload: Multipart) -> Result<Self, MultipartError> {
        println!("multipart");
        let mut multi_part: MultipartData<T> = MultipartData {
            files: None,
            data: None,
        };

        let mut hash: HashMap<String, String> = HashMap::new();
        println!("{:?}", hash);

        while let Some(item) = payload.next().await {
            println!("{:?}", item);
            let field: Option<Field> = match item {
                Ok(field) => Some(field),
                Err(e) => {
                    return Err(MultipartError::ValidationError(ValidationErrorStruct {
                        error: "Error al cargar el archivo".to_string(),
                        field: vec![format!("error al obtener field desde multiparte: {}", e)],
                        status_code: 400,
                    }))
                }
            };

            if let Some(mut field) = field {
                println!("field d: {:?}", field);
                // Early validation of Content-Length
                if let Some(len) = field.headers().get(CONTENT_LENGTH) {
                    if let Ok(s) = len.to_str() {
                        if let Ok(len) = s.parse::<u64>() {
                            if len > 3 * 1024 * 1024 * 1024 {
                                return Err(MultipartError::FileChargeError);
                            }
                        }
                    }
                }

                let content_type_header = field.headers().get("content-type");
                println!("content{:?}", content_type_header);
                let content_type = field.content_type();
                let content_type_max = content_type.type_().as_str().to_string();
                println!("field:{}", content_type.to_string());

                if content_type.to_string() == "application/octet-stream"
                    || content_type.to_string() == "application/json"
                    || content_type.to_string() == "text/csv"
                {
                    println!("content header: {:?}", content_type_header);
                    if let Some(head) = content_type_header {
                        if head == "application/octet-stream"
                            || head == "application/json"
                            || head == "text/csv"
                        {
                            println!("{:?}", head);
                            let mut byte = BytesMut::with_capacity(1024 * 1024 * 1024); // Pre-allocate 1GB
                            println!("es imagen");

                            let mut image_name: String = "".to_string();
                            let mut extension: String = "".to_string();

                            if let Some(content_disposition) =
                                field.headers().get(CONTENT_DISPOSITION)
                            {
                                let content_str_lossy =
                                    match std::str::from_utf8(content_disposition.as_bytes()) {
                                        Ok(content_str) => content_str.to_string(),
                                        Err(_) => {
                                            String::from_utf8_lossy(content_disposition.as_bytes())
                                                .to_string()
                                        }
                                    };
                                if let Some(name_field) = content_str_lossy.find("filename") {
                                    let start = name_field + 10;
                                    if let Some(end) = content_str_lossy[start..].find('"') {
                                        image_name =
                                            content_str_lossy[start..start + end].to_string();
                                        let image_ptr_name = &image_name;
                                        if let Some(img) = image_ptr_name.rfind(".") {
                                            extension = image_ptr_name[img + 1..].to_string();
                                        } else {
                                            return Err(MultipartError::FileChargeError);
                                        }
                                    }
                                }
                            } else {
                                return Err(MultipartError::FileChargeError);
                            }
                            println!("i{}", image_name);
                            println!("e{}", extension);

                            // Process chunks concurrently with timeout
                            let mut total_size = 0;
                            let timeout_duration = Duration::from_secs(600);
                            let result = time::timeout(timeout_duration, async {
                                let byte = Arc::new(Mutex::new(BytesMut::with_capacity(
                                    1024 * 1024 * 1024,
                                )));
                                let total_size = Arc::new(AtomicU64::new(0));

                                // Convert field error type to match our custom error
                                let field = field.map_err(|e| MultipartError::FileChargeError);

                                let result = field
                                    .try_for_each_concurrent(Some(16), |chunk| {
                                        let byte = Arc::clone(&byte);
                                        let total_size = Arc::clone(&total_size);
                                        async move {
                                            let chunk_size = chunk.len() as u64;
                                            let new_size = total_size
                                                .fetch_add(chunk_size, Ordering::SeqCst)
                                                + chunk_size;

                                            if new_size > 3 * 1024 * 1024 * 1024 {
                                                return Err(MultipartError::FileChargeError);
                                            }

                                            let mut byte_guard = byte.lock().await;
                                            byte_guard.extend_from_slice(&chunk);
                                            Ok(())
                                        }
                                    })
                                    .await;

                                // Extract the bytes from the Mutex
                                let byte = Arc::try_unwrap(byte).unwrap().into_inner();
                                (result, byte)
                            })
                            .await;
                            let content_type_string = content_type_max;

                            match result {
                                Ok((Ok(()), byte)) => {
                                    println!("File processed successfully");
                                    // Use the byte value here
                                    if !byte.is_empty() {
                                        let preload_file = PreLoadFile {
                                            file_name: image_name.clone(),
                                            file_data: byte,
                                            extension,
                                            content_type: content_type_string,
                                        };
                                        if multi_part.files.as_ref().is_none() {
                                            multi_part.files = Some(Vec::new());
                                        }
                                        if multi_part
                                            .files
                                            .as_ref()
                                            .is_some_and(|files| files.len() < 3)
                                        {
                                            println!("cargando file");
                                            multi_part.files.as_mut().unwrap().push(preload_file);
                                        }
                                    }
                                }
                                Ok((Err(e), b)) => return (Err(MultipartError::FileChargeError)),
                                Err(_) => return Err(MultipartError::FileChargeError), // Timeout
                            }
                        }
                    } else {
                        println!("desestructurizando");
                        let mut name_field_pre: String = "".to_string();
                        if let Some(content_disposition) =
                            &mut field.headers().get(CONTENT_DISPOSITION)
                        {
                            if let Ok(content_str) = content_disposition.to_str() {
                                if let Some(name_field) = content_str.find("name") {
                                    let start = name_field + 6;
                                    if let Some(end) = content_str[start..].find('"') {
                                        name_field_pre =
                                            content_str[start..start + end].to_string();
                                    }
                                }
                            }
                        }
                        let mut byte = BytesMut::new();
                        while let Some(chunk) = field.next().await {
                            match chunk {
                                Ok(chunk) => byte.extend_from_slice(&chunk),
                                Err(_) => {
                                    return Err(MultipartError::ValidationError(
                                        ValidationErrorStruct::new(vec![
                                            "error al obtener field desde multiparte".to_string(),
                                        ]),
                                    ))
                                }
                            }
                        }
                        let mut value = "".to_string();
                        if let Ok(data) = String::from_utf8(byte.to_vec()) {
                            value = data;
                        }
                        hash.insert(name_field_pre, value);
                    }
                } else {
                }
            }
        }
        let data_struc: Result<Option<T>, Vec<String>> = hash.try_from_hashmap();
        if let Err(err) = data_struc {
            println!("error inicial{:?}", err);
            return Err(MultipartError::ValidationError(ValidationErrorStruct::new(
                err,
            )));
        }
        let data_struc = data_struc.unwrap();
        multi_part.data = data_struc;
        println!("paso el multiparte");
        return Ok(multi_part);
    }

    pub fn get_data(&self) -> Option<&T> {
        self.data.as_ref()
    }
    pub fn get_files(&self) -> Option<&Vec<PreLoadFile>> {
        self.files.as_ref()
    }
    pub fn take_files(&mut self) -> Option<Vec<PreLoadFile>> {
        self.files.take()
    }
}
pub trait FileCharge {
    fn insert_file(&self) -> Result<File, std::io::Error>;
    fn delete_file(&self) -> Result<(), std::io::Error>;
}

impl FileCharge for PreLoadFile {
    fn insert_file(&self) -> Result<File, std::io::Error> {
        let mut file = File::create(&self.file_name)?;
        file.write_all(&self.file_data)?;
        Ok(file)
    }
    fn delete_file(&self) -> Result<(), std::io::Error> {
        std::fs::remove_file(&self.file_name)?;
        Ok(())
    }
}

impl<T, Err: ErrorRenderer> FromRequest<Err> for MultipartData<T>
where
    T: Default + Validate + DeserializeOwned,
{
    type Error = MultipartError;

    async fn from_request(req: &HttpRequest, payload: &mut Payload) -> Result<Self, Self::Error> {
        // Use the FromRequest implementation for Multipart to get the raw data
        println!("multipart1");
        let multipart = Multipart::new(req.headers(), payload.take());

        // Process the multipart data using the Multipart instance and extract data for T
        let data = match MultipartData::<T>::new(multipart).await {
            Ok(data) => data,
            Err(err) => {
                return Err(err);
            }
        };

        Ok(data)
    }
}
