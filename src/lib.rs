mod error;
#[allow(dead_code)] // Frontend API is consumed by the next implementation task.
mod oracle;
pub use error::FormatError;
pub const UPSTREAM_REVISION: &str = "b571298b6fc95bc863548f118354c8d077792f6f";
