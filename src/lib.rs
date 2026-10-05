mod document;
mod error;
mod layout;
mod oracle;
pub use error::FormatError;
pub const UPSTREAM_REVISION: &str = "b571298b6fc95bc863548f118354c8d077792f6f";

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
pub fn format(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    if !(1..=8).contains(&options.indent_width) || !(20..=240).contains(&options.line_width) {
        return Err(FormatError::InvalidOptions(
            "indent width must be 1..=8 and line width 20..=240".into(),
        ));
    }
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
        let candidate = layout::layout(source, &analysis, options, conservative);
        if preserves_source(source, &analysis, &candidate)? {
            return Ok(candidate);
        }
    }
    Err(FormatError::Validation(
        "neither preferred nor conservative layout preserved source syntax and tokens".into(),
    ))
}

fn preserves_source(
    source: &str,
    original: &oracle::Analysis,
    candidate: &str,
) -> Result<bool, FormatError> {
    let formatted = match oracle::analyze(candidate) {
        Ok(analysis) => analysis,
        Err(FormatError::Syntax { .. }) => return Ok(false),
        Err(error) => return Err(error),
    };
    let tape_matches =
        original.tokens.len() == formatted.tokens.len()
            && original.tokens.iter().zip(&formatted.tokens).all(|(a, b)| {
                a.kind == b.kind && source[a.start..a.end] == candidate[b.start..b.end]
            });
    Ok(tape_matches && oracle::equivalent(source, candidate)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_lexical_join_uses_verified_conservative_layout() {
        let source = "let x=1 .field";
        let options = FormatOptions::default();
        let analysis = oracle::analyze(source).unwrap();
        let preferred = layout::layout(source, &analysis, &options, false);
        assert!(!preserves_source(source, &analysis, &preferred).unwrap());
        let output = format(source, &options).unwrap();
        assert_eq!(output, "let x=1 .field\n");
        assert!(preserves_source(source, &analysis, &output).unwrap());
        assert_eq!(format(&output, &options).unwrap(), output);
    }

    #[test]
    fn tape_validation_rejects_moved_comments_and_literal_respelling() {
        for (source, candidate) in [
            ("let x = 1 ## note ##", "## note ## let x = 1"),
            ("let x = 'same'", "let x = \"same\""),
        ] {
            assert!(oracle::equivalent(source, candidate).unwrap());
            assert!(
                !preserves_source(source, &oracle::analyze(source).unwrap(), candidate).unwrap()
            );
        }
    }
}
