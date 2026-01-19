use std::collections::HashMap;

use axum::extract::multipart::Field;

use crate::{error::ApiError, s3::FileObject};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FileType {
    ImageJPEG,
    ImagePNG,
    ImageGIF,
    ImageWebP,
    VideoMP4,
    VideoWebM,
    AudioMP3,
    AudioWebM,
    AudioOGG,
    AudioWAV,
    DocumentPDF,
    Any,
}

impl From<&str> for FileType {
    fn from(s: &str) -> Self {
        match s {
            "image/jpeg" => FileType::ImageJPEG,
            "image/png" => FileType::ImagePNG,
            "image/gif" => FileType::ImageGIF,
            "image/webp" => FileType::ImageWebP,
            "video/mp4" => FileType::VideoMP4,
            "video/webm" => FileType::VideoWebM,
            "audio/mp3" => FileType::AudioMP3,
            "audio/webm" => FileType::AudioWebM,
            "audio/ogg" => FileType::AudioOGG,
            "audio/wav" => FileType::AudioWAV,
            "application/pdf" => FileType::DocumentPDF,
            _ => FileType::Any,
        }
    }
}

impl From<FileType> for &str {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::ImageJPEG => "image/jpeg",
            FileType::ImagePNG => "image/png",
            FileType::ImageGIF => "image/gif",
            FileType::ImageWebP => "image/webp",
            FileType::VideoMP4 => "video/mp4",
            FileType::VideoWebM => "video/webm",
            FileType::AudioMP3 => "audio/mp3",
            FileType::AudioWebM => "audio/webm",
            FileType::AudioOGG => "audio/ogg",
            FileType::AudioWAV => "audio/wav",
            FileType::DocumentPDF => "application/pdf",
            FileType::Any => "application/octet-stream",
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<ApiError> for GuardError {
    fn into(self) -> ApiError {
        match self {
            GuardError::FileTypeNotAllowed => {
                ApiError::BadRequest("File type not allowed".to_string())
            }
            GuardError::WrongContentType => ApiError::BadRequest("Wrong content type".to_string()),
            GuardError::UnknownFileType => ApiError::BadRequest("Unknown file type".to_string()),
            GuardError::MissingFileExtension => {
                ApiError::BadRequest("Missing file extension".to_string())
            }
            GuardError::WrongFileExtension => {
                ApiError::BadRequest("Wrong file extension".to_string())
            }
            GuardError::InternalServerError(e) => ApiError::InternalServerError(e),
            GuardError::NoGuardFound => ApiError::InternalServerError("No guard found".to_string()),
        }
    }
}

#[derive(Debug)]
pub enum GuardError {
    FileTypeNotAllowed,
    WrongContentType,
    UnknownFileType,
    MissingFileExtension,
    WrongFileExtension,
    NoGuardFound,
    InternalServerError(String),
}

#[derive(Clone, Debug)]
pub struct Guard {
    allowed_file_types: Vec<FileType>,
}

pub struct Guards {
    map: HashMap<String, Guard>,
}

impl Guard {
    pub fn new(allowed_file_types: Vec<FileType>) -> Self {
        Self { allowed_file_types }
    }

    pub async fn check(&self, field: Field<'_>, file_name: &str) -> Result<FileObject, GuardError> {
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let chunk_data = field
            .bytes()
            .await
            .map_err(|e| GuardError::InternalServerError(e.to_string()))?
            .to_vec();

        if self.allowed_file_types.contains(&FileType::Any) {
            return Ok(FileObject {
                data: chunk_data,
                content_type,
            });
        }

        let kind = infer::get(&chunk_data);

        match kind {
            Some(kind) => {
                let inferred_content_type: FileType = kind.mime_type().into();
                if !self.allowed_file_types.contains(&inferred_content_type) {
                    return Err(GuardError::FileTypeNotAllowed);
                }

                let inferred_content_type: &str = inferred_content_type.into();

                if inferred_content_type != content_type {
                    return Err(GuardError::WrongContentType);
                }

                let extension = file_name.split('.').next_back();
                if extension.is_none() {
                    return Err(GuardError::MissingFileExtension);
                }
                // We just checked that the extension is not None, so we can unwrap safely
                if extension.expect("Extension should be set") != kind.extension() {
                    return Err(GuardError::WrongFileExtension);
                }
            }
            None => {
                return Err(GuardError::UnknownFileType);
            }
        }

        Ok(FileObject {
            content_type,
            data: chunk_data,
        })
    }
}

pub struct GuardsBuilder {
    map: HashMap<String, Guard>,
}

impl GuardsBuilder {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn add(&mut self, destination: &str, guard: Guard) -> &mut Self {
        self.map.insert(destination.to_string(), guard);
        self
    }

    pub fn build(&self) -> Guards {
        Guards {
            map: self.map.clone(),
        }
    }
}

impl Guards {
    pub async fn check(
        &self,
        destination: &str,
        file_name: &str,
        field: Field<'_>,
    ) -> Result<FileObject, GuardError> {
        let guard = self.map.get(destination);
        let file = match guard {
            Some(guard) => guard.check(field, file_name).await,
            None => Err(GuardError::NoGuardFound),
        }?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use axum::{Router, extract::Multipart, response::IntoResponse, routing::post};
    use axum_test::TestServer;

    use crate::storage::handlers::put_object::tests::build_multipart;

    use super::*;

    #[tokio::test]
    async fn test_guard_happy_path() {
        let buf: &[u8] = &[0xFF, 0xD8, 0xFF, 0xAA];
        const FILE_NAME: &str = "index.jpg";
        const CONTENT_TYPE: &str = "image/jpeg";

        let form = build_multipart(buf, FILE_NAME, CONTENT_TYPE);

        async fn handle_happy_path(mut multipart: Multipart) -> impl IntoResponse {
            let field = multipart
                .next_field()
                .await
                .expect("Invalid field")
                .expect("Invalid field");
            let guards = GuardsBuilder::new()
                .add(
                    "test",
                    Guard {
                        allowed_file_types: vec![FileType::ImageJPEG],
                    },
                )
                .build();

            let file = guards.check("test", "index.jpg", field).await;
            insta::assert_debug_snapshot!(file);
        }

        let app = Router::new().route("/test/index.jpg", post(handle_happy_path));
        let client = TestServer::new(app).expect("Axum test server creation failed");

        let response = client.post("/test/index.jpg").multipart(form).await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_guard_any_files() {
        let buf: &[u8] = "test file".as_bytes();
        const FILE_NAME: &str = "index.html";
        const CONTENT_TYPE: &str = "text/html";

        let form = build_multipart(buf, FILE_NAME, CONTENT_TYPE);

        async fn handle_any_files(mut multipart: Multipart) -> impl IntoResponse {
            let field = multipart
                .next_field()
                .await
                .expect("Invalid field")
                .expect("Invalid field");
            let guards = GuardsBuilder::new()
                .add(
                    "test",
                    Guard {
                        allowed_file_types: vec![FileType::Any],
                    },
                )
                .build();

            let file = guards.check("test", "index.html", field).await;
            insta::assert_debug_snapshot!(file);
        }

        let app = Router::new().route("/test/index.html", post(handle_any_files));
        let client = TestServer::new(app).expect("Axum test server creation failed");

        let response = client.post("/test/index.html").multipart(form).await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_guard_file_confusion() {
        let buf: &[u8] = "<svg id='x' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='1337' height='1337'><image href='1' onerror='alert(window.origin)' /></svg>".as_bytes();
        const FILE_NAME: &str = "index.svg";
        const CONTENT_TYPE: &str = "application/octet-stream";

        let form = build_multipart(buf, FILE_NAME, CONTENT_TYPE);

        async fn handle_file_confusion(mut multipart: Multipart) -> impl IntoResponse {
            let field = multipart
                .next_field()
                .await
                .expect("Invalid field")
                .expect("Invalid field");
            let guards = GuardsBuilder::new()
                .add(
                    "test",
                    Guard {
                        allowed_file_types: vec![FileType::ImagePNG],
                    },
                )
                .build();

            let file = guards.check("test", "index.svg", field).await;
            insta::assert_debug_snapshot!(file);
        }

        let app = Router::new().route("/test/index.svg", post(handle_file_confusion));
        let client = TestServer::new(app).expect("Axum test server creation failed");

        let response = client.post("/test/index.svg").multipart(form).await;

        response.assert_status_ok();
    }
}
