use crate::config::{LayoutFlags, Resolver};
use revofmt::{IndentStyle, MAX_SOURCE_BYTES, UPSTREAM_REVISION, format};
use std::{
    ffi::OsString,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const HELP: &str = "revofmt [OPTIONS] [FILE]
revofmt --check [OPTIONS] [FILE...]
revofmt --write [OPTIONS] FILE...

Read stdin when FILE is absent or is -. Print formatted source by default.

Options:
  --check             Report changed inputs without editing
  --write             Atomically replace changed files, preserving permissions
  --indent-width N    Columns per indentation level (1 through 8; default 2)
  --indent-style S    Indent with space or tab (default space)
  --line-width N      Target display columns (20 through 240; default 80)
  --max-blank-lines N Consecutive blank lines kept (0 through 8; default 1)
  --stdin-filepath P  Find revofmt.toml from P when reading stdin
  --prefer-config     Ignore layout flags when revofmt.toml applies
  --no-config         Do not search for revofmt.toml
  --help              Print this help
  --version           Print formatter version and pinned syntax revision
  --                  Treat following arguments as file paths

Layout flags override revofmt.toml, found in each input's directory or its
nearest ancestor directory. A tab counts as --indent-width columns.
Print mode accepts one input. --check and --write are mutually exclusive.
Write mode rejects stdin and symlinks. Exit codes: 0 success, 1 check
differences, 2 usage/I/O/syntax/validation error. Diagnostics go to stderr.
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Print,
    Check,
    Write,
}

struct Arguments {
    mode: Mode,
    flags: LayoutFlags,
    prefer_config: bool,
    no_config: bool,
    stdin_filepath: Option<PathBuf>,
    inputs: Vec<PathBuf>,
}

enum Command {
    Help,
    Version,
    Format(Arguments),
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let mut mode = Mode::Print;
    // Each layout flag is range-checked as it is parsed, even when
    // `--prefer-config` later ignores it, so the resolver only sees valid flags.
    let mut flags = LayoutFlags::default();
    let mut prefer_config = false;
    let mut no_config = false;
    let mut stdin_filepath = None;
    let mut inputs = Vec::new();
    let mut literal_paths = false;
    while let Some(arg) = args.next() {
        if literal_paths {
            inputs.push(PathBuf::from(arg));
            continue;
        }
        match arg.to_str() {
            Some("--") => literal_paths = true,
            Some("--help") => return Ok(Command::Help),
            Some("--version") => return Ok(Command::Version),
            Some("--check" | "--write") => {
                let selected = if arg == "--check" {
                    Mode::Check
                } else {
                    Mode::Write
                };
                if mode != Mode::Print && mode != selected {
                    return Err("--check and --write are mutually exclusive".into());
                }
                mode = selected;
            }
            Some("--indent-width" | "--line-width") => {
                let name = arg.to_str().unwrap();
                let value = args
                    .next()
                    .ok_or_else(|| format!("{name} requires a value"))?;
                let value = value
                    .to_str()
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| format!("{name} requires a positive integer"))?;
                flags = if name == "--indent-width" {
                    flags.with_indent_width(value)?
                } else {
                    flags.with_line_width(value)?
                };
            }
            Some("--indent-style") => {
                let style = match args.next().as_deref().and_then(|v| v.to_str()) {
                    Some("space") => IndentStyle::Space,
                    Some("tab") => IndentStyle::Tab,
                    _ => return Err("--indent-style requires space or tab".into()),
                };
                flags = flags.with_indent_style(style)?;
            }
            Some("--max-blank-lines") => {
                let value = args
                    .next()
                    .as_deref()
                    .and_then(|v| v.to_str())
                    .and_then(|v| v.parse::<usize>().ok())
                    .ok_or("--max-blank-lines requires a non-negative integer")?;
                flags = flags.with_max_blank_lines(value)?;
            }
            Some("--stdin-filepath") => {
                stdin_filepath = Some(PathBuf::from(
                    args.next().ok_or("--stdin-filepath requires a path")?,
                ));
            }
            Some("--prefer-config") => prefer_config = true,
            Some("--no-config") => no_config = true,
            _ if arg != "-" && arg.to_string_lossy().starts_with('-') => {
                return Err(format!("unrecognized option: {}", arg.to_string_lossy()));
            }
            _ => inputs.push(PathBuf::from(arg)),
        }
    }
    if mode == Mode::Write && (inputs.is_empty() || inputs.iter().any(|p| p == Path::new("-"))) {
        return Err("--write requires file paths and cannot read stdin".into());
    }
    if inputs.is_empty() {
        inputs.push(PathBuf::from("-"));
    }
    if stdin_filepath.is_some() && inputs.iter().any(|p| p != Path::new("-")) {
        return Err("--stdin-filepath requires stdin input".into());
    }
    if inputs.len() > 1 && (mode == Mode::Print || inputs.iter().any(|p| p == Path::new("-"))) {
        return Err("print mode and stdin accept exactly one input".into());
    }
    Ok(Command::Format(Arguments {
        mode,
        flags,
        prefer_config,
        no_config,
        stdin_filepath,
        inputs,
    }))
}

pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<u8, String> {
    match parse(args)? {
        Command::Help => print_output(HELP),
        Command::Version => print_output(&format!(
            "revofmt {} (Revo {UPSTREAM_REVISION})\n",
            env!("CARGO_PKG_VERSION")
        )),
        Command::Format(args) => execute(&args),
    }
}

fn execute(args: &Arguments) -> Result<u8, String> {
    // One resolver per run, so each configuration file is read at most once.
    let mut resolver = Resolver::new(args.flags, args.prefer_config, !args.no_config);
    let stdin_filepath = args.stdin_filepath.as_deref();
    match args.mode {
        Mode::Print => {
            let path = &args.inputs[0];
            let (_, formatted) = read_and_format(path, &mut resolver, stdin_filepath)?;
            print_output(&formatted)
        }
        Mode::Check => {
            let mut changed = false;
            let mut failed = false;
            for path in &args.inputs {
                match read_and_format(path, &mut resolver, stdin_filepath) {
                    Ok((source, formatted)) if source != formatted => {
                        eprintln!("{}: requires formatting", input_name(path));
                        changed = true;
                    }
                    Ok(_) => {}
                    Err(message) => {
                        eprintln!("revofmt: {message}");
                        failed = true;
                    }
                }
            }
            Ok(if failed { 2 } else { u8::from(changed) })
        }
        Mode::Write => {
            // Validate the entire batch, including each input's configuration,
            // before any replacement.
            write_batch::PreparedBatch::prepare(&args.inputs, &mut resolver)?.apply()?;
            Ok(0)
        }
    }
}

fn print_output(source: &str) -> Result<u8, String> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(source.as_bytes())
        .and_then(|()| stdout.flush())
        .map_err(|error| format!("stdout: {error}"))?;
    Ok(0)
}

fn input_name(path: &Path) -> String {
    if path == Path::new("-") {
        "stdin".into()
    } else {
        path.display().to_string()
    }
}

/// Read one input, resolve its options and format it. Stdin discovers
/// configuration only from `stdin_filepath`; files discover from their own path.
fn read_and_format(
    path: &Path,
    resolver: &mut Resolver,
    stdin_filepath: Option<&Path>,
) -> Result<(String, String), String> {
    let stdin = path == Path::new("-");
    let source = if stdin {
        read_source(io::stdin().lock())
    } else {
        File::open(path).and_then(read_source)
    }
    .map_err(|error| format!("{}: {error}", input_name(path)))?;
    let options = resolver
        .options(if stdin { stdin_filepath } else { Some(path) })
        .map_err(|error| format!("{}: {error}", input_name(path)))?;
    let formatted =
        format(&source, &options).map_err(|error| format!("{}: {error}", input_name(path)))?;
    Ok((source, formatted))
}

fn read_source(reader: impl Read) -> io::Result<String> {
    // Read one extra byte to distinguish an exact-sized input from truncation.
    // Decode only after the size check, so a split UTF-8 sequence is a size error.
    let mut bytes = Vec::new();
    reader
        .take((MAX_SOURCE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("input complexity limit exceeded: source bytes ({MAX_SOURCE_BYTES})"),
        ));
    }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// The complete `--write` lifecycle: prepare, then apply.
///
/// Preparation validates the whole batch and modifies nothing. Only a prepared
/// batch can be applied, and applying consumes it. Each replacement is atomic on
/// its own; the batch is not a transaction, so a later failure leaves earlier
/// replacements in place and the error names them.
mod write_batch {
    use super::read_and_format;
    use crate::config::Resolver;
    use std::{
        fs::{self, File, OpenOptions, Permissions},
        io::{self, Write},
        os::unix::fs::OpenOptionsExt,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    /// Changed files in input order, each with its formatted text and the
    /// permissions captured during preparation.
    pub(super) struct PreparedBatch {
        replacements: Vec<Replacement>,
    }

    struct Replacement {
        path: PathBuf,
        source: String,
        permissions: Permissions,
    }

    impl PreparedBatch {
        /// Check every path in order: it is a regular file, it reads, its options
        /// resolve and it formats. Unchanged files are skipped. Fails with the
        /// first problem, before any file is replaced.
        pub(super) fn prepare(paths: &[PathBuf], resolver: &mut Resolver) -> Result<Self, String> {
            let mut replacements = Vec::new();
            for path in paths {
                let permissions = regular_file_permissions(path)?;
                let (source, formatted) = read_and_format(path, resolver, None)?;
                if source != formatted {
                    replacements.push(Replacement {
                        path: path.clone(),
                        source: formatted,
                        permissions,
                    });
                }
            }
            Ok(Self { replacements })
        }

        /// Replace each changed file in input order. A failure names the failing
        /// path and the paths already replaced, which stay replaced.
        pub(super) fn apply(self) -> Result<(), String> {
            let mut completed: Vec<PathBuf> = Vec::new();
            for replacement in self.replacements {
                if let Err(error) = atomic_replace(&replacement) {
                    let mut message = format!("{}: {error}", replacement.path.display());
                    if !completed.is_empty() {
                        message.push_str("\nWrites completed before this failure:");
                        for path in completed {
                            message.push_str(&format!("\n  {}", path.display()));
                        }
                    }
                    return Err(message);
                }
                completed.push(replacement.path);
            }
            Ok(())
        }
    }

    fn regular_file_permissions(path: &Path) -> Result<Permissions, String> {
        let metadata =
            fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "{}: --write requires a regular file and rejects symlinks",
                path.display()
            ));
        }
        Ok(metadata.permissions())
    }

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn create_temporary(destination: &Path) -> io::Result<(PathBuf, File)> {
        let directory = destination.parent().unwrap_or_else(|| Path::new("."));
        for _ in 0..100 {
            let path = directory.join(format!(
                ".revofmt-{}-{}.tmp",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not create a unique temporary file",
        ))
    }

    fn atomic_replace(replacement: &Replacement) -> Result<(), String> {
        let (temporary, mut file) =
            create_temporary(&replacement.path).map_err(|error| error.to_string())?;
        let result = (|| {
            file.write_all(replacement.source.as_bytes())
                .map_err(|error| error.to_string())?;
            file.set_permissions(replacement.permissions.clone())
                .map_err(|error| error.to_string())?;
            file.flush()
                .and_then(|()| file.sync_all())
                .map_err(|error| error.to_string())?;
            drop(file);
            // Check the type again in case it changed during batch preparation.
            regular_file_permissions(&replacement.path)?;
            fs::rename(&temporary, &replacement.path).map_err(|error| error.to_string())
        })();
        if let Err(error) = result {
            return match fs::remove_file(&temporary) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!(
                    "{error}; could not remove temporary file {}: {cleanup}",
                    temporary.display()
                )),
            };
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::LayoutFlags;

        struct Scratch(PathBuf);

        impl Scratch {
            fn new() -> Self {
                let directory = std::env::temp_dir()
                    .join(format!("revofmt-write-batch-{}", std::process::id()));
                fs::create_dir(&directory).unwrap();
                Self(directory)
            }
        }

        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        fn names(directory: &Path) -> Vec<String> {
            let mut names = fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect::<Vec<_>>();
            names.sort();
            names
        }

        #[test]
        fn a_destination_that_stops_being_a_file_fails_after_earlier_replacements() {
            let scratch = Scratch::new();
            let first = scratch.0.join("first.rv");
            let second = scratch.0.join("second.rv");
            fs::write(&first, "let x=1").unwrap();
            fs::write(&second, "let y=2").unwrap();
            let mut resolver = Resolver::new(LayoutFlags::default(), false, false);
            let batch =
                PreparedBatch::prepare(&[first.clone(), second.clone()], &mut resolver).unwrap();
            assert_eq!(fs::read(&first).unwrap(), b"let x=1");
            assert_eq!(fs::read(&second).unwrap(), b"let y=2");
            assert_eq!(names(&scratch.0), ["first.rv", "second.rv"]);

            let moved = scratch.0.join("moved.rv");
            fs::rename(&second, &moved).unwrap();
            fs::create_dir(&second).unwrap();
            let error = batch.apply().unwrap_err();

            assert_eq!(fs::read(&first).unwrap(), b"let x = 1\n");
            assert_eq!(fs::read(&moved).unwrap(), b"let y=2");
            // The recheck's own diagnostic already names the path, which the
            // batch prefixes again.
            assert_eq!(
                error,
                format!(
                    "{second}: {second}: --write requires a regular file and rejects symlinks\n\
                     Writes completed before this failure:\n  {first}",
                    first = first.display(),
                    second = second.display(),
                )
            );
            assert_eq!(names(&scratch.0), ["first.rv", "moved.rv", "second.rv"]);
            assert!(names(&second).is_empty());
        }
    }
}
