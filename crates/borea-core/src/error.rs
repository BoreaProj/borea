use std::fmt;
#[derive(Debug)]
pub struct BoreaError {
    pub code: &'static str,
    pub message: String,
}
impl BoreaError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("E_CONFIG", message)
    }
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self::new("E_UNSUPPORTED", reason)
    }
    pub fn exit_code(&self) -> i32 {
        match self.code {
            "E_UNSUPPORTED" => 3,
            "E_STEP" | "E_IO" | "E_RUNTIME_EXEC" => 4,
            "E_VERIFY" => 5,
            code if code.starts_with("E_RUNTIME") => 3,
            _ => 2,
        }
    }
}
impl fmt::Display for BoreaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for BoreaError {}
impl From<std::io::Error> for BoreaError {
    fn from(e: std::io::Error) -> Self {
        Self::new("E_IO", e.to_string())
    }
}
impl From<serde_json::Error> for BoreaError {
    fn from(e: serde_json::Error) -> Self {
        Self::new("E_JSON", e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, BoreaError>;
