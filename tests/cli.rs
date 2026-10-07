use std::{
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "revofmt-cli-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, source: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, source).unwrap();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn invoke(args: &[&OsStr], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_revofmt"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn run(args: &[&str], stdin: &str) -> Output {
    invoke(&args.iter().map(OsStr::new).collect::<Vec<_>>(), stdin)
}

fn status(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn contains_path(output: &Output, path: &Path) {
    assert!(String::from_utf8_lossy(&output.stderr).contains(&*path.to_string_lossy()));
}

#[test]
fn stdin_and_explicit_stdin_print_source_only() {
    for args in [vec![], vec!["-"]] {
        let output = run(&args, "let x=1");
        status(&output, 0);
        assert_eq!(output.stdout, b"let x = 1\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn stress_syntax_survives_stdin_file_check_and_write() {
    let source = include_str!("fixtures/stress/syntax.rv");
    for (line_width, indent_width) in [("20", "1"), ("80", "2"), ("240", "8")] {
        for crlf in [false, true] {
            let source = if crlf {
                source.replace('\n', "\r\n")
            } else {
                source.to_owned()
            };
            let args = ["--line-width", line_width, "--indent-width", indent_width];
            let output = run(&args, &source);
            status(&output, 0);
            assert!(output.stderr.is_empty());
            let formatted = String::from_utf8(output.stdout).unwrap();
            // The library corpus tests separately verify the actual syntax and
            // token/comment tape. Here check the complete CLI transport.
            assert_eq!(
                formatted,
                revofmt::format(
                    &source,
                    &revofmt::FormatOptions {
                        line_width: line_width.parse().unwrap(),
                        indent_width: indent_width.parse().unwrap(),
                    }
                )
                .unwrap()
            );
            let repeated = run(&args, &formatted);
            status(&repeated, 0);
            assert_eq!(repeated.stdout, formatted.as_bytes());
            assert!(repeated.stderr.is_empty());

            let dir = TempDir::new();
            let path = dir.file("stress.rv", &source);
            let mut file_args: Vec<_> = args.iter().map(OsStr::new).collect();
            file_args.push(path.as_os_str());
            let printed = invoke(&file_args, "");
            status(&printed, 0);
            assert_eq!(printed.stdout, formatted.as_bytes());
            assert_eq!(fs::read_to_string(&path).unwrap(), source);

            file_args.insert(0, OsStr::new("--check"));
            let checked = invoke(&file_args, "");
            status(&checked, 1);
            assert!(checked.stdout.is_empty());
            assert_eq!(fs::read_to_string(&path).unwrap(), source);

            file_args[0] = OsStr::new("--write");
            let written = invoke(&file_args, "");
            status(&written, 0);
            assert!(written.stdout.is_empty());
            assert_eq!(fs::read_to_string(&path).unwrap(), formatted);

            file_args[0] = OsStr::new("--check");
            let checked = invoke(&file_args, "");
            status(&checked, 0);
            assert!(checked.stdout.is_empty());
            assert!(checked.stderr.is_empty());
        }
    }
}

#[test]
fn file_print_check_write_and_check_again() {
    let dir = TempDir::new();
    let path = dir.file("source.rv", "let x=1");
    let output = invoke(&[path.as_os_str()], "");
    status(&output, 0);
    assert_eq!(output.stdout, b"let x = 1\n");
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read(&path).unwrap(), b"let x=1");
    let output = invoke(&[OsStr::new("--check"), path.as_os_str()], "");
    status(&output, 1);
    assert!(output.stdout.is_empty());
    contains_path(&output, &path);
    assert_eq!(fs::read(&path).unwrap(), b"let x=1");
    let output = invoke(&[OsStr::new("--write"), path.as_os_str()], "");
    status(&output, 0);
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(&path).unwrap(), b"let x = 1\n");
    let output = invoke(&[OsStr::new("--check"), path.as_os_str()], "");
    status(&output, 0);
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn help_version_and_option_overrides() {
    let help = run(&["--help"], "");
    status(&help, 0);
    let help = String::from_utf8(help.stdout).unwrap();
    for option in ["--write", "--check", "--indent-width", "--line-width", "--"] {
        assert!(help.contains(option));
    }
    let version = run(&["--version"], "");
    status(&version, 0);
    let version = String::from_utf8(version.stdout).unwrap();
    assert!(version.contains(env!("CARGO_PKG_VERSION")));
    assert!(version.contains(revofmt::UPSTREAM_REVISION));
    let output = run(
        &["--indent-width", "4", "--line-width", "24"],
        "do\nprint(first_argument, second_argument)\nend",
    );
    status(&output, 0);
    assert_eq!(
        output.stdout,
        b"do\n    print(\n        first_argument,\n        second_argument\n    )\nend\n"
    );
    for (indent, width) in [("1", "20"), ("8", "240")] {
        status(
            &run(
                &["--indent-width", indent, "--line-width", width],
                "let x=1",
            ),
            0,
        );
    }
}

#[test]
fn rejects_invalid_arguments_without_source() {
    for args in [
        vec!["--check", "--write", "file.rv"],
        vec!["--indent-width"],
        vec!["--line-width"],
        vec!["--indent-width", "0"],
        vec!["--indent-width", "9"],
        vec!["--line-width", "19"],
        vec!["--line-width", "241"],
        vec!["--line-width", "invalid"],
        vec!["--unknown"],
        vec!["--write"],
        vec!["--write", "-"],
        vec!["--write", "a.rv", "-"],
        vec!["a.rv", "b.rv"],
        vec!["-", "a.rv"],
        vec!["--check", "-", "a.rv"],
    ] {
        let output = run(&args, "let x=1");
        status(&output, 2);
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn check_supports_stdin() {
    for args in [vec!["--check"], vec!["--check", "-"]] {
        let output = run(&args, "let x=1");
        status(&output, 1);
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("stdin"));
        status(&run(&args, "let x = 1\n"), 0);
    }
}

#[test]
fn literal_flag_filename_after_separator() {
    let dir = TempDir::new();
    dir.file("--check", "let x=1");
    let output = Command::new(env!("CARGO_BIN_EXE_revofmt"))
        .args(["--", "--check"])
        .current_dir(&dir.0)
        .output()
        .unwrap();
    status(&output, 0);
    assert_eq!(output.stdout, b"let x = 1\n");
}

#[test]
fn malformed_input_never_prints_or_changes_files() {
    let output = run(&[], "let x =");
    status(&output, 2);
    assert!(output.stdout.is_empty());
    let dir = TempDir::new();
    let path = dir.file("bad.rv", "let x =");
    for args in [
        vec![path.as_os_str()],
        vec![OsStr::new("--write"), path.as_os_str()],
    ] {
        let output = invoke(&args, "");
        status(&output, 2);
        assert!(output.stdout.is_empty());
        contains_path(&output, &path);
        assert_eq!(fs::read(&path).unwrap(), b"let x =");
    }
}

#[test]
fn excessive_syntax_returns_an_error_without_output_or_writes() {
    let deep = format!("{}1{}", "(".repeat(3000), ")".repeat(3000));
    let sources = [
        deep.clone(),
        format!("{}1", "(".repeat(3000)),
        format!("{}1", "not ".repeat(3000)),
        std::iter::repeat_n("1", 6000).collect::<Vec<_>>().join("+"),
        std::iter::repeat_n("1", 1500).collect::<Vec<_>>().join("%"),
        std::iter::repeat_n("1", 1500).collect::<Vec<_>>().join(">"),
        format!("\"value #{{{deep}}}\""),
        format!("`{deep}`"),
        format!("a{}", ".field".repeat(4000)),
        format!("a{}", ".f".repeat(1500)),
        format!("a{}", ":f()".repeat(760)),
        format!("{}foo.end{}", "do\n".repeat(300), "\nend".repeat(300)),
        format!("{}foo.end{}", "do\n".repeat(336), "\nend".repeat(336)),
        format!(
            "{}{}",
            "match x | _ => ".repeat(330),
            std::iter::repeat_n("1", 799).collect::<Vec<_>>().join("+")
        ),
    ];
    let dir = TempDir::new();
    let first = dir.file("first.rv", "let x=1");
    let bad = dir.file("complex.rv", "");
    let mut failures = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let output = run(&[], source);
        if output.status.code() != Some(2)
            || !output.stdout.is_empty()
            || !String::from_utf8_lossy(&output.stderr).contains("input complexity limit")
        {
            failures.push(format!(
                "case {index}: {:?}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        fs::write(&bad, source).unwrap();
        let output = invoke(
            &[OsStr::new("--write"), first.as_os_str(), bad.as_os_str()],
            "",
        );
        if output.status.code() != Some(2) || !output.stdout.is_empty() {
            failures.push(format!("write case {index}: {:?}", output.status));
        }
        assert_eq!(
            fs::read(&first).unwrap(),
            b"let x=1",
            "{}",
            failures.join("\n")
        );
        assert_eq!(fs::read_to_string(&bad).unwrap(), *source);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
}

#[test]
fn bounded_reads_reject_oversized_utf8_without_output_or_batch_writes() {
    // The last permitted read byte lands in the middle of this UTF-8 character.
    let oversized = "é".repeat(revofmt::MAX_SOURCE_BYTES / 2 + 1);
    let output = run(&[], &oversized);
    status(&output, 2);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("source bytes"));
    let dir = TempDir::new();
    let first = dir.file("first.rv", "let x=1");
    let bad = dir.file("large.rv", &oversized);
    let output = invoke(
        &[OsStr::new("--write"), first.as_os_str(), bad.as_os_str()],
        "",
    );
    status(&output, 2);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("source bytes"));
    assert_eq!(fs::read(&first).unwrap(), b"let x=1");
    assert_eq!(fs::read_to_string(&bad).unwrap(), oversized);
    let stable = format!("'{}'\n", "x".repeat(revofmt::MAX_SOURCE_BYTES - 3));
    let boundary = dir.file("boundary.rv", &stable);
    status(
        &invoke(&[OsStr::new("--check"), boundary.as_os_str()], ""),
        0,
    );
    let no_space_for_newline = format!("'{}'", "x".repeat(revofmt::MAX_SOURCE_BYTES - 2));
    fs::write(&boundary, &no_space_for_newline).unwrap();
    let output = invoke(&[boundary.as_os_str()], "");
    status(&output, 2);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("source bytes"));
    assert_eq!(fs::read_to_string(&boundary).unwrap(), no_space_for_newline);
}

#[test]
fn multi_write_prevalidates_every_input() {
    let dir = TempDir::new();
    let first = dir.file("first.rv", "let x=1");
    let bad = dir.file("bad.rv", "let x =");
    let output = invoke(
        &[OsStr::new("--write"), first.as_os_str(), bad.as_os_str()],
        "",
    );
    status(&output, 2);
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(&first).unwrap(), b"let x=1");
    assert_eq!(fs::read(&bad).unwrap(), b"let x =");
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
}

#[test]
fn invalid_utf8_is_an_io_error_and_prevents_batch_writes() {
    let dir = TempDir::new();
    let first = dir.file("first.rv", "let x=1");
    let bad = dir.0.join("bad.rv");
    fs::write(&bad, [0xff, 0xfe]).unwrap();
    for args in [
        vec![bad.as_os_str()],
        vec![OsStr::new("--write"), first.as_os_str(), bad.as_os_str()],
    ] {
        let output = invoke(&args, "");
        status(&output, 2);
        assert!(output.stdout.is_empty());
        contains_path(&output, &bad);
        assert_eq!(fs::read(&first).unwrap(), b"let x=1");
        assert_eq!(fs::read(&bad).unwrap(), [0xff, 0xfe]);
    }
}

#[test]
fn missing_paths_and_check_errors_take_precedence_over_differences() {
    let dir = TempDir::new();
    let first = dir.file("first.rv", "let x=1");
    let bad = dir.file("bad.rv", "let x =");
    let missing = dir.0.join("missing.rv");
    for path in [&bad, &missing] {
        let output = invoke(
            &[OsStr::new("--check"), first.as_os_str(), path.as_os_str()],
            "",
        );
        status(&output, 2);
        assert!(output.stdout.is_empty());
        contains_path(&output, &first);
        contains_path(&output, path);
    }
    for args in [
        vec![missing.as_os_str()],
        vec![
            OsStr::new("--write"),
            first.as_os_str(),
            missing.as_os_str(),
        ],
    ] {
        let output = invoke(&args, "");
        status(&output, 2);
        assert!(output.stdout.is_empty());
        contains_path(&output, &missing);
        assert_eq!(fs::read(&first).unwrap(), b"let x=1");
    }
}

#[test]
fn multi_write_and_multi_check() {
    let dir = TempDir::new();
    let first = dir.file("first.rv", "let x=1");
    let second = dir.file("second.rv", "let y=2");
    let args = [first.as_os_str(), second.as_os_str()];
    let output = invoke(&[OsStr::new("--check"), args[0], args[1]], "");
    status(&output, 1);
    contains_path(&output, &first);
    contains_path(&output, &second);
    let output = invoke(&[OsStr::new("--write"), args[0], args[1]], "");
    status(&output, 0);
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(&first).unwrap(), b"let x = 1\n");
    assert_eq!(fs::read(&second).unwrap(), b"let y = 2\n");
    status(&invoke(&[OsStr::new("--check"), args[0], args[1]], ""), 0);
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn write_preserves_permissions_and_skips_unchanged_files() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let dir = TempDir::new();
    let changed = dir.file("changed.rv", "let x=1");
    let unchanged = dir.file("unchanged.rv", "let y = 2\n");
    fs::set_permissions(&changed, fs::Permissions::from_mode(0o751)).unwrap();
    let before = fs::metadata(&unchanged).unwrap();
    let output = invoke(
        &[
            OsStr::new("--write"),
            changed.as_os_str(),
            unchanged.as_os_str(),
        ],
        "",
    );
    status(&output, 0);
    assert_eq!(
        fs::metadata(&changed).unwrap().permissions().mode() & 0o7777,
        0o751
    );
    let after = fs::metadata(&unchanged).unwrap();
    assert_eq!(before.ino(), after.ino());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
}

#[cfg(unix)]
#[test]
fn write_rejects_symlinks_and_nonregular_files_before_writes() {
    use std::os::unix::fs::symlink;
    let dir = TempDir::new();
    let first = dir.file("first.rv", "let x=1");
    let link = dir.0.join("link.rv");
    symlink(&first, &link).unwrap();
    for path in [&link, &dir.0] {
        let output = invoke(
            &[OsStr::new("--write"), first.as_os_str(), path.as_os_str()],
            "",
        );
        status(&output, 2);
        assert!(output.stdout.is_empty());
        contains_path(&output, path);
        assert_eq!(fs::read(&first).unwrap(), b"let x=1");
    }
    assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
}

#[cfg(unix)]
#[test]
fn later_io_failure_reports_completed_paths_and_leaves_no_temporary_files() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new();
    let first = dir.file("first.rv", "let x=1");
    let blocked_dir = dir.0.join("blocked");
    fs::create_dir(&blocked_dir).unwrap();
    let second = blocked_dir.join("second.rv");
    fs::write(&second, "let y=2").unwrap();
    fs::set_permissions(&blocked_dir, fs::Permissions::from_mode(0o555)).unwrap();
    let output = invoke(
        &[OsStr::new("--write"), first.as_os_str(), second.as_os_str()],
        "",
    );
    fs::set_permissions(&blocked_dir, fs::Permissions::from_mode(0o755)).unwrap();
    status(&output, 2);
    assert!(output.stdout.is_empty());
    contains_path(&output, &first);
    contains_path(&output, &second);
    assert!(String::from_utf8_lossy(&output.stderr).contains("completed"));
    assert_eq!(fs::read(&first).unwrap(), b"let x = 1\n");
    assert_eq!(fs::read(&second).unwrap(), b"let y=2");
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    assert_eq!(fs::read_dir(&blocked_dir).unwrap().count(), 1);
}
