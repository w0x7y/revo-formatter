mod corpus;
mod formatting;

use crate::{FormatOptions, format, oracle};

fn assert_preserved_and_idempotent(source: &str, output: &str, options: &FormatOptions) {
    assert!(
        oracle::analyze(source).unwrap().preserves(output).unwrap(),
        "{source:?}"
    );
    assert_eq!(format(output, options).unwrap(), output, "{source:?}");
}
