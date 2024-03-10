#[derive(Debug)]
pub enum CustomErrors {
    IoError(std::io::Error), // FileIO errors
    SerdeError(serde_json::Error), // Serde de/serialisation errors
}


impl From<std::io::Error> for CustomErrors{
    fn from(err: std::io::Error) -> Self {
        CustomErrors::IoError(err)
    }
}

impl From<serde_json::Error> for CustomErrors{
    fn from(err: serde_json::Error) -> Self {
        CustomErrors::SerdeError(err)
    }
}

impl From<std::io::ErrorKind> for CustomErrors {
    fn from(err_kind: std::io::ErrorKind) -> Self {
        CustomErrors::IoError(err_kind.into())
    }
}

impl std::fmt::Display for CustomErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CustomErrors::IoError(e) => write!(f, "{}", e),
            CustomErrors::SerdeError(e) => write!(f, "{}", e)
        }
    }
}

impl std::error::Error for CustomErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CustomErrors::IoError(e) => e.source(),
            CustomErrors::SerdeError(e) => e.source(),
        }
    }
}

impl CustomErrors{
    pub fn convert(e: serde_json::Error) -> Self {
        match e.io_error_kind() {
            Some(e1) => CustomErrors::IoError(e1.into()), // io::Error
            None => CustomErrors::SerdeError(e) // Serde Error
        }
    }
}