#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    InvalidOptions(String),
    Syntax { message: String, offset: usize },
    Validation(String),
}
impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOptions(message) | Self::Validation(message) => f.write_str(message),
            Self::Syntax { message, offset } => write!(f, "{message} at byte {offset}"),
        }
    }
}
impl std::error::Error for FormatError {}
