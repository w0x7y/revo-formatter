//! `revofmt.toml` parsing and per-input discovery for the command line.
//!
//! Layout flags and file keys are each partial: a [`Layout`] holds only the
//! values that were supplied. The [`Resolver`] merges them over the library
//! defaults according to the precedence table in the configuration design.

use revofmt::{FormatOptions, IndentStyle};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
};

pub(crate) const FILE_NAME: &str = "revofmt.toml";

/// Largest configuration file read, in bytes. One more byte is read to detect
/// a larger file without buffering it.
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

/// Layout values that were supplied by a flag or a configuration file.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    pub(crate) indent_width: Option<usize>,
    pub(crate) line_width: Option<usize>,
    pub(crate) indent_style: Option<IndentStyle>,
    pub(crate) max_blank_lines: Option<usize>,
}

impl Layout {
    /// Replace each `base` value for which this layout supplies one.
    pub(crate) fn over(self, base: FormatOptions) -> FormatOptions {
        FormatOptions {
            indent_width: self.indent_width.unwrap_or(base.indent_width),
            line_width: self.line_width.unwrap_or(base.line_width),
            indent_style: self.indent_style.unwrap_or(base.indent_style),
            max_blank_lines: self.max_blank_lines.unwrap_or(base.max_blank_lines),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayout {
    indent_width: Option<usize>,
    line_width: Option<usize>,
    indent_style: Option<String>,
    max_blank_lines: Option<usize>,
}

/// Parse the contents of a configuration file. Messages omit the file path.
pub(crate) fn parse(text: &str) -> Result<Layout, String> {
    let raw: RawLayout = toml::from_str(text).map_err(|error| {
        let line = error
            .span()
            .and_then(|span| text.get(..span.start))
            .map(|before| before.bytes().filter(|&byte| byte == b'\n').count() + 1);
        match line {
            Some(line) => format!("{} (line {line})", error.message()),
            None => error.message().to_string(),
        }
    })?;
    let indent_style = match raw.indent_style.as_deref() {
        None => None,
        Some("space") => Some(IndentStyle::Space),
        Some("tab") => Some(IndentStyle::Tab),
        Some(other) => {
            return Err(format!(
                "invalid indent_style `{other}`, expected \"space\" or \"tab\""
            ));
        }
    };
    Ok(Layout {
        indent_width: raw.indent_width,
        line_width: raw.line_width,
        indent_style,
        max_blank_lines: raw.max_blank_lines,
    })
}

type Found = Option<(PathBuf, Layout)>;

/// Resolves the options for each input from flags and the nearest
/// configuration file, caching discovery results per directory.
pub(crate) struct Resolver {
    flags: Layout,
    prefer_config: bool,
    discover: bool,
    cache: HashMap<PathBuf, Result<Found, String>>,
}

impl Resolver {
    pub(crate) fn new(flags: Layout, prefer_config: bool, discover: bool) -> Self {
        Self {
            flags,
            prefer_config,
            discover,
            cache: HashMap::new(),
        }
    }

    /// Options for one input. `anchor` is the input path or `--stdin-filepath`;
    /// `None` skips discovery. Errors name the configuration path.
    pub(crate) fn options(&mut self, anchor: Option<&Path>) -> Result<FormatOptions, String> {
        let found = match anchor {
            Some(anchor) if self.discover => self.discover(anchor)?,
            _ => None,
        };
        let defaults = FormatOptions::default();
        let options = match &found {
            Some((_, file)) if self.prefer_config => file.over(defaults),
            Some((_, file)) => self.flags.over(file.over(defaults)),
            None => self.flags.over(defaults),
        };
        options.validate().map_err(|error| match &found {
            Some((path, _)) => at(path, error),
            None => error.to_string(),
        })?;
        Ok(options)
    }

    fn discover(&mut self, anchor: &Path) -> Result<Found, String> {
        let start = start_directory(anchor)?;
        let mut visited = Vec::new();
        let mut result = Ok(None);
        for dir in start.ancestors() {
            if let Some(cached) = self.cache.get(dir) {
                result = cached.clone();
                break;
            }
            visited.push(dir.to_path_buf());
            match read_in(dir) {
                Ok(None) => {}
                found => {
                    result = found;
                    break;
                }
            }
        }
        for dir in visited {
            self.cache.insert(dir, result.clone());
        }
        result
    }
}

fn at(path: &Path, message: impl std::fmt::Display) -> String {
    format!("{}: {message}", path.display())
}

/// The absolute directory where discovery starts for `anchor`: the canonical
/// form of the deepest existing ancestor of its parent directory. Directories
/// that do not exist cannot hold a configuration, and walking their lexical
/// parents could leave the real tree through `..` or a symbolic link.
fn start_directory(anchor: &Path) -> Result<PathBuf, String> {
    let parent = anchor.parent().unwrap_or(anchor);
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let joined = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("cannot determine the current directory: {error}"))?
            .join(parent)
    };
    for dir in joined.ancestors() {
        if let Ok(canonical) = fs::canonicalize(dir) {
            return Ok(canonical);
        }
    }
    Ok(joined)
}

/// Read the configuration file in `dir`, if any. A missing entry is `Ok(None)`.
fn read_in(dir: &Path) -> Result<Found, String> {
    let path = dir.join(FILE_NAME);
    match fs::symlink_metadata(&path) {
        Ok(_) => {}
        // A start directory that is really a file, or an ancestor that is,
        // cannot contain a configuration.
        Err(error) if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
            return Ok(None);
        }
        Err(error) => return Err(at(&path, format!("cannot read: {error}"))),
    }
    let metadata =
        fs::metadata(&path).map_err(|error| at(&path, format!("cannot read: {error}")))?;
    if !metadata.is_file() {
        return Err(at(&path, "not a regular file"));
    }
    let mut bytes = Vec::new();
    File::open(&path)
        .and_then(|file| file.take(MAX_CONFIG_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|error| at(&path, format!("cannot read: {error}")))?;
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err(at(
            &path,
            format!("larger than the {MAX_CONFIG_BYTES} byte limit"),
        ));
    }
    let text = String::from_utf8(bytes).map_err(|_| at(&path, "not valid UTF-8"))?;
    let layout = parse(&text).map_err(|message| at(&path, message))?;
    Ok(Some((path, layout)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use revofmt::{FormatOptions, IndentStyle};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    /// A canonical temporary directory, removed on drop.
    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "revofmt-config-{}-{}",
                std::process::id(),
                NEXT_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
        fn path(&self, relative: &str) -> PathBuf {
            self.0.join(relative)
        }
        fn file(&self, relative: &str, contents: impl AsRef<[u8]>) -> PathBuf {
            let path = self.path(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, contents).unwrap();
            path
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn resolver(flags: Layout) -> Resolver {
        Resolver::new(flags, false, true)
    }

    #[test]
    fn parses_every_key() {
        let text =
            "indent_width = 4\nline_width = 100\nindent_style = \"tab\"\nmax_blank_lines = 2\n";
        assert_eq!(
            parse(text).unwrap(),
            Layout {
                indent_width: Some(4),
                line_width: Some(100),
                indent_style: Some(IndentStyle::Tab),
                max_blank_lines: Some(2),
            }
        );
    }

    #[test]
    fn empty_and_crlf_files_parse() {
        assert_eq!(parse("").unwrap(), Layout::default());
        assert_eq!(
            parse("# comment\r\nline_width = 90\r\n")
                .unwrap()
                .line_width,
            Some(90)
        );
    }

    #[test]
    fn rejects_unknown_keys_wrong_types_and_styles() {
        for text in [
            "indnet_width = 4",
            "indent_width = \"4\"",
            "indent_width = 4.0",
            "indent_width = -1",
            "indent_style = \"tabs\"",
            "[layout]\nindent_width = 4",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn deep_nesting_is_an_error_on_a_two_mib_thread() {
        let text = format!("indent_width = {}", "[".repeat(60_000));
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || parse(&text))
            .unwrap()
            .join()
            .unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn balanced_deep_nesting_is_a_recursion_error_on_a_two_mib_thread() {
        let inputs = [
            format!(
                "indent_width = {}{}",
                "[".repeat(60_000),
                "]".repeat(60_000)
            ),
            format!(
                "indent_width = {}1{}",
                "{a=".repeat(20_000),
                "}".repeat(20_000)
            ),
        ];
        for text in inputs {
            let error = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || parse(&text))
                .unwrap()
                .join()
                .unwrap()
                .unwrap_err();
            assert!(error.contains("recurs"), "{error}");
        }
    }

    #[test]
    fn deeply_nested_key_paths_are_errors_on_a_two_mib_thread() {
        const DEPTH: usize = 30_000;
        let path = vec!["a"; DEPTH].join(".");
        let inputs = [
            ("dotted key", format!("{path} = 1\n")),
            ("table header", format!("[{path}]\n")),
            ("array of tables", format!("[[{path}]]\n")),
            ("inline dotted key", format!("x = {{ {path} = 1 }}\n")),
            ("nested dotted key", format!("indent_width.{path} = 1\n")),
        ];
        // Each stays under the 64 KiB bound of the file reader.
        for (name, text) in inputs {
            assert!(text.len() < 65_536, "{name}");
            let error = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || parse(&text))
                .unwrap()
                .join()
                .unwrap()
                .unwrap_err();
            assert!(error.contains("recurs"), "{name}: {error}");
        }
    }

    #[test]
    fn nearest_configuration_wins() {
        let dir = TempDir::new();
        dir.file(FILE_NAME, "line_width = 100\n");
        dir.file("sub/revofmt.toml", "line_width = 60\n");
        fs::create_dir_all(dir.path("sub/deeper")).unwrap();
        let mut resolver = resolver(Layout::default());
        let deep = resolver
            .options(Some(&dir.path("sub/deeper/a.rv")))
            .unwrap();
        assert_eq!(deep.line_width, 60);
        let top = resolver.options(Some(&dir.path("b.rv"))).unwrap();
        assert_eq!(top.line_width, 100);
    }

    #[test]
    fn precedence_follows_the_spec_table() {
        let dir = TempDir::new();
        dir.file(FILE_NAME, "indent_style = \"tab\"\nline_width = 24\n");
        let anchor = dir.path("a.rv");
        let flags = Layout {
            line_width: Some(80),
            max_blank_lines: Some(0),
            ..Layout::default()
        };
        let summary = |options: FormatOptions| {
            (
                options.indent_style,
                options.line_width,
                options.max_blank_lines,
            )
        };

        let default = Resolver::new(flags, false, true)
            .options(Some(&anchor))
            .unwrap();
        assert_eq!(summary(default), (IndentStyle::Tab, 80, 0));

        let preferred = Resolver::new(flags, true, true)
            .options(Some(&anchor))
            .unwrap();
        assert_eq!(summary(preferred), (IndentStyle::Tab, 24, 1));

        let undiscovered = Resolver::new(flags, false, false)
            .options(Some(&anchor))
            .unwrap();
        assert_eq!(summary(undiscovered), (IndentStyle::Space, 80, 0));

        let unanchored = Resolver::new(flags, false, true).options(None).unwrap();
        assert_eq!(summary(unanchored), (IndentStyle::Space, 80, 0));

        let unanchored_preferred = Resolver::new(flags, true, true).options(None).unwrap();
        assert_eq!(summary(unanchored_preferred), (IndentStyle::Space, 80, 0));
    }

    /// Fail when a configuration above the temporary directory would make a
    /// "no configuration" expectation meaningless.
    fn assert_no_configuration_above_temp() {
        let temp = std::env::temp_dir().canonicalize().unwrap();
        for ancestor in temp.ancestors() {
            let stray = ancestor.join(FILE_NAME);
            assert!(
                fs::symlink_metadata(&stray).is_err(),
                "{} exists above the temporary directory; remove it so the \
                 no-configuration test is meaningful",
                stray.display()
            );
        }
    }

    #[test]
    fn no_configuration_uses_flags_and_defaults() {
        assert_no_configuration_above_temp();
        let dir = TempDir::new();
        fs::create_dir_all(dir.path("empty")).unwrap();
        let flags = Layout {
            indent_width: Some(3),
            max_blank_lines: Some(2),
            ..Layout::default()
        };
        for prefer_config in [false, true] {
            let options = Resolver::new(flags, prefer_config, true)
                .options(Some(&dir.path("empty/a.rv")))
                .unwrap();
            assert_eq!(options, flags.over(FormatOptions::default()));
        }
    }

    #[test]
    fn out_of_range_values_name_the_configuration() {
        let dir = TempDir::new();
        let config = dir.file(FILE_NAME, "max_blank_lines = 9\n");
        let error = resolver(Layout::default())
            .options(Some(&dir.path("a.rv")))
            .unwrap_err();
        assert!(
            error.starts_with(&format!("{}: ", config.display())),
            "{error}"
        );
        assert!(error.contains("max blank lines"), "{error}");
    }

    #[test]
    fn invalid_syntax_names_the_configuration_and_line() {
        let dir = TempDir::new();
        let config = dir.file(FILE_NAME, "line_width = 90\nindnet_width = 4\n");
        let error = resolver(Layout::default())
            .options(Some(&dir.path("a.rv")))
            .unwrap_err();
        assert!(
            error.starts_with(&format!("{}: ", config.display())),
            "{error}"
        );
        assert!(error.contains("indnet_width"), "{error}");
        assert!(error.contains("line 2"), "{error}");
    }

    #[test]
    fn unreadable_entries_are_errors() {
        use std::os::unix::fs::symlink;

        type Setup = fn(&TempDir);
        let cases: [(&str, Setup); 4] = [
            ("too large", |dir| {
                dir.file(FILE_NAME, "#".repeat(65_537));
            }),
            ("invalid UTF-8", |dir| {
                dir.file(FILE_NAME, [0xff]);
            }),
            ("directory", |dir| {
                fs::create_dir(dir.path(FILE_NAME)).unwrap();
            }),
            ("dangling symlink", |dir| {
                symlink("missing.toml", dir.path(FILE_NAME)).unwrap();
            }),
        ];
        for (name, setup) in cases {
            let dir = TempDir::new();
            setup(&dir);
            let error = resolver(Layout::default())
                .options(Some(&dir.path("a.rv")))
                .unwrap_err();
            let config = dir.path(FILE_NAME);
            assert!(
                error.starts_with(&format!("{}: ", config.display())),
                "{name}: {error}"
            );
        }

        let dir = TempDir::new();
        dir.file("real.toml", "line_width = 33\n");
        symlink("real.toml", dir.path(FILE_NAME)).unwrap();
        let options = resolver(Layout::default())
            .options(Some(&dir.path("a.rv")))
            .unwrap();
        assert_eq!(options.line_width, 33);
    }

    #[test]
    fn a_file_of_exactly_the_size_limit_is_read() {
        let dir = TempDir::new();
        let mut text = String::from("line_width = 41\n");
        text.push_str(&"#".repeat(65_536 - text.len()));
        assert_eq!(text.len(), 65_536);
        dir.file(FILE_NAME, text);
        let options = resolver(Layout::default())
            .options(Some(&dir.path("a.rv")))
            .unwrap();
        assert_eq!(options.line_width, 41);
    }

    #[test]
    fn results_are_cached_per_directory() {
        let dir = TempDir::new();
        let config = dir.file(FILE_NAME, "line_width = 50\n");
        let mut resolver = resolver(Layout::default());
        let first = resolver.options(Some(&dir.path("a.rv"))).unwrap();
        fs::remove_file(config).unwrap();
        let second = resolver.options(Some(&dir.path("b.rv"))).unwrap();
        assert_eq!((first.line_width, second.line_width), (50, 50));
        // Descendants share the ancestor's cached result.
        fs::create_dir_all(dir.path("sub")).unwrap();
        let nested = resolver.options(Some(&dir.path("sub/c.rv"))).unwrap();
        assert_eq!(nested.line_width, 50);
    }

    #[test]
    fn errors_are_cached_too() {
        let dir = TempDir::new();
        let config = dir.file(FILE_NAME, "line_width = \"wide\"\n");
        let mut resolver = resolver(Layout::default());
        let first = resolver.options(Some(&dir.path("a.rv"))).unwrap_err();
        fs::write(&config, "line_width = 50\n").unwrap();
        let second = resolver.options(Some(&dir.path("b.rv"))).unwrap_err();
        assert_eq!(first, second);
    }

    #[test]
    fn a_missing_subdirectory_of_a_configured_project_finds_its_configuration() {
        let dir = TempDir::new();
        dir.file(FILE_NAME, "line_width = 70\n");
        let options = resolver(Layout::default())
            .options(Some(&dir.path("not/created/a.rv")))
            .unwrap();
        assert_eq!(options.line_width, 70);
    }

    #[test]
    fn a_file_used_as_a_directory_falls_back_to_its_ancestors() {
        let dir = TempDir::new();
        dir.file(FILE_NAME, "line_width = 70\n");
        dir.file("plain.txt", "not a directory");
        let options = resolver(Layout::default())
            .options(Some(&dir.path("plain.txt/a.rv")))
            .unwrap();
        assert_eq!(options.line_width, 70);
    }

    #[test]
    fn dot_dot_through_a_missing_directory_is_judged_by_the_real_location() {
        assert_no_configuration_above_temp();
        let dir = TempDir::new();
        dir.file("project_a/revofmt.toml", "indent_style = \"tab\"\n");
        fs::create_dir_all(dir.path("project_b")).unwrap();
        let anchor = dir.path("project_a/../project_b/new/x.rv");

        let options = resolver(Layout::default()).options(Some(&anchor)).unwrap();
        assert_eq!(options, FormatOptions::default());

        dir.file("project_b/revofmt.toml", "line_width = 55\n");
        let options = resolver(Layout::default()).options(Some(&anchor)).unwrap();
        assert_eq!(options.line_width, 55);
        assert_eq!(options.indent_style, IndentStyle::Space);
    }

    #[test]
    fn a_symlinked_file_uses_the_configuration_of_the_directory_holding_the_link() {
        use std::os::unix::fs::symlink;

        assert_no_configuration_above_temp();
        let dir = TempDir::new();
        dir.file("project/revofmt.toml", "indent_style = \"tab\"\n");
        dir.file("elsewhere/a.rv", "let x = 1\n");
        symlink(dir.path("elsewhere/a.rv"), dir.path("project/a.rv")).unwrap();
        let options = resolver(Layout::default())
            .options(Some(&dir.path("project/a.rv")))
            .unwrap();
        assert_eq!(options.indent_style, IndentStyle::Tab);
        let options = resolver(Layout::default())
            .options(Some(&dir.path("elsewhere/a.rv")))
            .unwrap();
        assert_eq!(options, FormatOptions::default());
    }

    #[test]
    fn a_symlink_out_of_the_project_is_judged_by_the_real_location() {
        use std::os::unix::fs::symlink;

        assert_no_configuration_above_temp();
        let dir = TempDir::new();
        dir.file("project/revofmt.toml", "indent_style = \"tab\"\n");
        fs::create_dir_all(dir.path("outside")).unwrap();
        symlink(dir.path("outside"), dir.path("project/link")).unwrap();
        let anchors = [
            dir.path("project/link/new/x.rv"),
            dir.path("project/../outside/new/x.rv"),
            dir.path("project/link/x.rv"),
        ];

        for anchor in &anchors {
            let options = resolver(Layout::default()).options(Some(anchor)).unwrap();
            assert_eq!(options, FormatOptions::default(), "{}", anchor.display());
        }

        dir.file("outside/revofmt.toml", "line_width = 55\n");
        for anchor in &anchors {
            let options = resolver(Layout::default()).options(Some(anchor)).unwrap();
            assert_eq!(options.line_width, 55, "{}", anchor.display());
            assert_eq!(options.indent_style, IndentStyle::Space);
        }
    }
}
