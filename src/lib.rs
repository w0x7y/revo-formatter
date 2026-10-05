mod document;
mod error;
mod layout;
mod layout_index;
mod oracle;
pub use error::FormatError;
pub const UPSTREAM_REVISION: &str = "b571298b6fc95bc863548f118354c8d077792f6f";
/// Maximum source length, in UTF-8 bytes. Syntax complexity has additional limits.
pub const MAX_SOURCE_BYTES: usize = 256 * 1024;

/// Layout settings. Width is a soft limit for opaque tokens and sensitive syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub indent_width: usize,
    pub line_width: usize,
}
impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_width: 2,
            line_width: 80,
        }
    }
}

/// Format without changing token bytes or syntax, modulo source coordinates.
/// Procedural macros that inspect offsets, lines or columns can observe formatting.
/// Literal contents remain opaque. Invalid input produces no formatted candidate.
/// Inputs exceeding the documented resource limits return `FormatError::Validation`.
pub fn format(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    if !(1..=8).contains(&options.indent_width) || !(20..=240).contains(&options.line_width) {
        return Err(FormatError::InvalidOptions(
            "indent width must be 1..=8 and line width 20..=240".into(),
        ));
    }
    oracle::preflight(source)?;
    let mut original = source.to_owned();
    // A returned result must be a fixed point of the complete choice algorithm,
    // including conservative fallback. Never emit an unstable intermediate.
    for _ in 0..4 {
        let candidate = choose_layout(&original, options)?;
        if candidate == original {
            return Ok(candidate);
        }
        original = candidate;
    }
    Err(FormatError::Validation(
        "layout did not reach a stable result".into(),
    ))
}

fn choose_layout(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    let analysis = oracle::analyze(source)?;
    for conservative in [false, true] {
        let candidate = layout::layout(&analysis, options, conservative);
        if analysis.preserves(&candidate)? {
            return Ok(candidate);
        }
    }
    Err(FormatError::Validation(
        "neither preferred nor conservative layout preserved source syntax and tokens".into(),
    ))
}

#[cfg(test)]
mod preservation_tests {
    use super::*;

    #[test]
    fn unsafe_lexical_join_uses_verified_conservative_layout() {
        let source = "let x=1 .field";
        let options = FormatOptions::default();
        let analysis = oracle::analyze(source).unwrap();
        let preferred = layout::layout(&analysis, &options, false);
        assert!(!analysis.preserves(&preferred).unwrap());
        let output = format(source, &options).unwrap();
        assert_eq!(output, "let x=1 .field\n");
        assert!(analysis.preserves(&output).unwrap());
        assert_eq!(format(&output, &options).unwrap(), output);
    }
}

#[cfg(test)]
mod tests;
