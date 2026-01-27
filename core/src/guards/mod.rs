use std::collections::HashMap;

use crate::{error::ApiError, prefixes::Prefix, s3::FileObject};

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
            "audio/mpeg" => FileType::AudioMP3,
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
            FileType::AudioMP3 => "audio/mpeg",
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
            GuardError::NoGuardFound => ApiError::InternalServerError("No guard found".to_string()),
            GuardError::UnknownPrefix => ApiError::NotFound("Unknown prefix".to_string()),
        }
    }
}

#[derive(Debug)]
pub enum GuardError {
    FileTypeNotAllowed,
    WrongContentType,
    UnknownFileType,
    UnknownPrefix,
    NoGuardFound,
}

#[derive(Clone, Debug)]
pub struct Guard {
    allowed_file_types: Vec<FileType>,
}

pub struct Guards {
    map: HashMap<Prefix, Guard>,
}

impl Guard {
    pub fn new(allowed_file_types: Vec<FileType>) -> Self {
        Self { allowed_file_types }
    }

    pub fn check(
        &self,
        data: Vec<u8>,
        content_type: &str,
        _file_name: &str,
    ) -> Result<FileObject, GuardError> {
        let content_type = content_type.to_string();

        if self.allowed_file_types.contains(&FileType::Any) {
            return Ok(FileObject { data, content_type });
        }

        let kind = infer::get(&data);

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

                // let extension = file_name.split('.').next_back();
                // if extension.is_none() {
                //     return Err(GuardError::MissingFileExtension);
                // }
                // // We just checked that the extension is not None, so we can unwrap safely
                // if extension.expect("Extension should be set") != kind.extension() {
                //     return Err(GuardError::WrongFileExtension);
                // }
            }
            None => {
                return Err(GuardError::UnknownFileType);
            }
        }

        Ok(FileObject { content_type, data })
    }
}

pub struct GuardsBuilder {
    map: HashMap<Prefix, Guard>,
}

impl GuardsBuilder {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn add(&mut self, destination: Prefix, guard: Guard) -> &mut Self {
        self.map.insert(destination, guard);
        self
    }

    pub fn build(&self) -> Guards {
        Guards {
            map: self.map.clone(),
        }
    }
}

impl Guards {
    pub fn check(
        &self,
        destination: &str,
        file_name: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<FileObject, GuardError> {
        let prefix = Prefix::from(destination);
        if prefix == Prefix::Unknown {
            return Err(GuardError::UnknownPrefix);
        }
        let guard = self.map.get(&prefix);
        let file = match guard {
            Some(guard) => guard.check(data, content_type, file_name),
            None => Err(GuardError::NoGuardFound),
        }?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_happy_path() {
        let buf: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xAA];
        const FILE_NAME: &str = "index.jpg";
        const CONTENT_TYPE: &str = "image/jpeg";

        let guards = GuardsBuilder::new()
            .add(
                Prefix::ServerBanner,
                Guard {
                    allowed_file_types: vec![FileType::ImageJPEG],
                },
            )
            .build();

        let file = guards.check(Prefix::ServerBanner.as_str(), FILE_NAME, buf, CONTENT_TYPE);
        insta::assert_debug_snapshot!(file);
    }

    #[test]
    fn test_guard_any_files() {
        let buf: Vec<u8> = "test file".as_bytes().to_vec();
        const FILE_NAME: &str = "index.html";
        const CONTENT_TYPE: &str = "text/html";

        let guards = GuardsBuilder::new()
            .add(
                Prefix::ServerBanner,
                Guard {
                    allowed_file_types: vec![FileType::Any],
                },
            )
            .build();

        let file = guards.check(Prefix::ServerBanner.as_str(), FILE_NAME, buf, CONTENT_TYPE);
        insta::assert_debug_snapshot!(file);
    }

    #[test]
    fn test_guard_file_confusion() {
        let buf: Vec<u8> = "<svg id='x' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='1337' height='1337'><image href='1' onerror='alert(window.origin)' /></svg>".as_bytes().to_vec();
        const FILE_NAME: &str = "index.svg";
        const CONTENT_TYPE: &str = "application/octet-stream";

        let guards = GuardsBuilder::new()
            .add(
                Prefix::ServerBanner,
                Guard {
                    allowed_file_types: vec![FileType::ImagePNG],
                },
            )
            .build();

        let file = guards.check("test", FILE_NAME, buf, CONTENT_TYPE);
        insta::assert_debug_snapshot!(file);
    }
}
