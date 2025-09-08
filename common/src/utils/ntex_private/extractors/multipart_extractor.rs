use std::{collections::HashMap, fmt::Debug, fs::File, io::Write};

use crate::utils::traits::hashmap::HashMapToStruct;
use futures::StreamExt;
use ntex::{
    http::{header::CONTENT_DISPOSITION, Payload},
    util::BytesMut,
    web::{ErrorRenderer, FromRequest, HttpRequest},
};
use ntex_multipart::{Field, Multipart};
use serde::de::DeserializeOwned;
use validator::Validate;

use super::errors::{MultipartError, ValidationErrorStruct, ValidationFieldsErrorStruct};

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
        let mut multi_part: MultipartData<T> = MultipartData {
            files: None,
            data: None,
        };
        let mut hash: HashMap<String, String> = HashMap::new();
        while let Some(item) = payload.next().await {
            println!("{:?}", item);
            let field: Option<Field> = match item {
                Ok(field) => Some(field),
                Err(_) => {
                    return Err(MultipartError::ValidationError(ValidationErrorStruct {
                        error: "Error al cargar el archivo".to_string(),
                        field: vec!["error al obtener field desde multiparte".to_string()],
                        status_code: 400,
                    }))
                }
            };

            // if multi_part.multi_archive.is_none() {
            //     if let Some(archive_type) =
            //         MultiArchive::get_multi_archive(mime::IMAGE_PNG.type_().as_str())
            //     {
            //         multi_part.multi_archive = Some(archive_type);
            //     }
            // }
            //TODO implementar cantidad de bytes maxima como 500kb
            if let Some(mut field) = field {
                let content_type_header = field.headers().get("content-type");
                println!("{:?}", content_type_header);
                //primer content type
                let content_type = field.content_type();
                let content_type_max = content_type.type_().as_str().to_string();
                // let type_archive: String = match &multi_part.multi_archive {
                //     None => {
                //         if field.content_type().to_string() != "application/octet-stream" {
                //             if let Some(archive_type) = MultiArchive::get_multi_archive(&content_type_max) {
                //                 let archive_type_string = archive_type.get_type().to_string();
                //                 multi_part.multi_archive = Some(archive_type);
                //                 archive_type_string
                //             } else {
                //                 "".to_string()
                //             }
                //         } else {
                //             "".to_string()
                //         }
                //     }
                //     Some(a) => {
                //         let type_str = &content_type.to_string();
                //         if type_str != "application/octet-stream" {
                //             if content_type_max != a.get_type() {
                //                 return Err(MultipartError::FileChargeError);
                //             }
                //         }
                //         println!("{}", a.get_type());
                //         a.get_type().to_string()
                //     }
                // };
                //println!("{}", field.content_type().type_().as_str());
                //println!("{}", field.content_type());
                println!("field:{}", content_type.to_string());
                if content_type.to_string() == "application/octet-stream"
                    || content_type.to_string() == "application/json"
                {
                    println!("content header:   {:?}", content_type_header);
                    if let Some(head) = content_type_header {
                        if head == "application/octet-stream" || head == "application/json" {
                            println!("{:?}", head);
                            let mut byte = BytesMut::new();
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
                            while let Some(chunk_result) = field.next().await {
                                let chunk = match chunk_result {
                                    Ok(chunk) => chunk,
                                    Err(_) => return Err(MultipartError::FileChargeError),
                                };
                                byte.extend_from_slice(&chunk);
                            }
                            let content_type_string = content_type_max;
                            if !byte.is_empty() {
                                let preload_file = PreLoadFile {
                                    file_name: image_name.clone(),
                                    file_data: byte.clone(),
                                    extension: extension,
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
                                    multi_part.files.as_mut().unwrap().push(preload_file);
                                }
                            }
                        }
                    } else
                    //field.content_type().type_().as_str()
                    // == mime::APPLICATION_OCTET_STREAM.type_().as_str()
                    {
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
                }
            } else {
                return Err(MultipartError::FileChargeError); // Convertir &str a Box<dyn std::error::Error>
            }
        }
        println!("{:?}", hash);
        let data_struc: Result<Option<T>, Vec<String>> = hash.try_from_hashmap();
        if let Err(err) = data_struc {
            println!("error inicial{:?}", err);
            return Err(MultipartError::ValidationError(ValidationErrorStruct::new(
                err,
            )));
        }
        let data_struc = data_struc.unwrap();
        multi_part.data = data_struc;
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
