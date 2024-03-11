#[derive(Debug)]
pub enum Custom {
    IoError(std::io::Error),       // FileIO errors
    SerdeError(serde_json::Error), // Serde de/serialisation errors
}

impl From<std::io::Error> for Custom {
    fn from(err: std::io::Error) -> Self {
        Custom::IoError(err)
    }
}

impl From<std::io::ErrorKind> for Custom {
    fn from(err_kind: std::io::ErrorKind) -> Self {
        Custom::IoError(err_kind.into())
    }
}

impl std::fmt::Display for Custom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Custom::IoError(e) => write!(f, "{e}"),
            Custom::SerdeError(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Custom {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Custom::IoError(e) => e.source(),
            Custom::SerdeError(e) => e.source(),
        }
    }
}

impl From<serde_json::Error> for Custom {
    fn from(value: serde_json::Error) -> Self {
        match value.io_error_kind() {
            Some(e1) => Custom::IoError(e1.into()), // io::Error
            None => Custom::SerdeError(value),      // Serde Error
        }
    }
}
