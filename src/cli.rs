use revofmt::{FormatOptions, UPSTREAM_REVISION, format};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions, Permissions},
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const HELP: &str = "revofmt [OPTIONS] [FILE]
revofmt --check [OPTIONS] [FILE...]
revofmt --write [OPTIONS] FILE...

Read stdin when FILE is absent or is -. Print formatted source by default.

Options:
  --check             Report changed inputs without editing
  --write             Atomically replace changed files, preserving permissions
  --indent-width N    Spaces per indentation level (1 through 8; default 2)
  --line-width N      Target display columns (20 through 240; default 80)
  --help              Print this help
  --version           Print formatter version and pinned syntax revision
  --                  Treat following arguments as file paths

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
    options: FormatOptions,
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
    let mut options = FormatOptions::default();
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
                let (range, destination) = if name == "--indent-width" {
                    (1..=8, &mut options.indent_width)
                } else {
                    (20..=240, &mut options.line_width)
                };
                if !range.contains(&value) {
                    return Err(format!(
                        "{name} must be {} through {}",
                        range.start(),
                        range.end()
                    ));
                }
                *destination = value;
            }
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
    if inputs.len() > 1 && (mode == Mode::Print || inputs.iter().any(|p| p == Path::new("-"))) {
        return Err("print mode and stdin accept exactly one input".into());
    }
    Ok(Command::Format(Arguments {
        mode,
        options,
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
        Command::Format(args) => match args.mode {
            Mode::Print => {
                let path = &args.inputs[0];
                let (_, formatted) = read_and_format(path, &args.options)?;
                print_output(&formatted)
            }
            Mode::Check => {
                let mut changed = false;
                let mut failed = false;
                for path in &args.inputs {
                    match read_and_format(path, &args.options) {
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
            Mode::Write => write_files(&args),
        },
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

fn read_and_format(path: &Path, options: &FormatOptions) -> Result<(String, String), String> {
    let source = if path == Path::new("-") {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).map(|_| source)
    } else {
        fs::read_to_string(path)
    }
    .map_err(|error| format!("{}: {error}", input_name(path)))?;
    let formatted =
        format(&source, options).map_err(|error| format!("{}: {error}", input_name(path)))?;
    Ok((source, formatted))
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

struct Replacement {
    path: PathBuf,
    source: String,
    permissions: Permissions,
}

fn write_files(args: &Arguments) -> Result<u8, String> {
    // Validate the entire batch before any replacement. Later I/O failures can
    // still leave earlier replacements in place, so retain their paths.
    let mut replacements = Vec::new();
    for path in &args.inputs {
        let permissions = regular_file_permissions(path)?;
        let (source, formatted) = read_and_format(path, &args.options)?;
        if source != formatted {
            replacements.push(Replacement {
                path: path.clone(),
                source: formatted,
                permissions,
            });
        }
    }
    let mut completed: Vec<PathBuf> = Vec::new();
    for replacement in replacements {
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
    Ok(0)
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

    #[test]
    fn replacement_cleans_temporary_when_destination_is_no_longer_regular() {
        let directory = std::env::temp_dir().join(format!(
            "revofmt-replacement-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let destination = directory.join("source.rv");
        fs::create_dir(&destination).unwrap();
        let result = atomic_replace(&Replacement {
            path: destination.clone(),
            source: "let x = 1\n".into(),
            permissions: fs::metadata(&destination).unwrap().permissions(),
        });
        let remaining = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        fs::remove_dir_all(&directory).unwrap();
        assert!(result.unwrap_err().contains("regular file"));
        assert_eq!(remaining, vec![destination]);
    }
}
