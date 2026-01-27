#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Prefix {
    ServerPicture,
    ServerBanner,
    ProfilePicture,
    MessageAttachment,
    Unknown,
}

impl Prefix {
    pub fn as_str(&self) -> &str {
        match self {
            Prefix::ServerPicture => "server_picture",
            Prefix::ServerBanner => "server_banner",
            Prefix::ProfilePicture => "profile_picture",
            Prefix::MessageAttachment => "message_attachment",
            Prefix::Unknown => "unknown",
        }
    }
}

impl From<&str> for Prefix {
    fn from(s: &str) -> Self {
        match s {
            "server_picture" => Prefix::ServerPicture,
            "server_banner" => Prefix::ServerBanner,
            "profile_picture" => Prefix::ProfilePicture,
            "message_attachment" => Prefix::MessageAttachment,
            _ => Prefix::Unknown,
        }
    }
}
