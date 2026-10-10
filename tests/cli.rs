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
        // Inputs discover revofmt.toml in every ancestor, so a stray file above
        // the temporary directory would change unrelated results.
        let root = fs::canonicalize(std::env::temp_dir()).unwrap();
        for ancestor in root.ancestors() {
            assert!(
                !ancestor.join("revofmt.toml").exists(),
                "{} would configure every test input",
                ancestor.join("revofmt.toml").display()
            );
        }
        let path = root.join(format!(
            "revofmt-cli-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    /// Write `source` to `name`, creating missing parent directories.
    fn file(&self, name: &str, source: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn invoke_in(dir: Option<&Path>, args: &[&OsStr], stdin: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_revofmt"));
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn invoke(args: &[&OsStr], stdin: &str) -> Output {
    invoke_in(None, args, stdin)
}

fn run(args: &[&str], stdin: &str) -> Output {
    invoke(&args.iter().map(OsStr::new).collect::<Vec<_>>(), stdin)
}

/// Run with `dir` as the working directory.
fn run_in(dir: &Path, args: &[&str], stdin: &str) -> Output {
    invoke_in(
        Some(dir),
        &args.iter().map(OsStr::new).collect::<Vec<_>>(),
        stdin,
    )
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
fn upstream_syntax_errors_produce_no_output_and_preserve_write_batches() {
    for source in [
        "for i in 0 ..3 do i end",
        "for i in 0..2 ..6 do i end",
        "const t=1;print(\"#{t:d}\")",
    ] {
        let output = run(&[], source);
        status(&output, 2);
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());

        let dir = TempDir::new();
        let valid = dir.file("valid.rv", "let x=1");
        let invalid = dir.file("invalid.rv", source);
        let output = invoke(
            &[
                OsStr::new("--write"),
                valid.as_os_str(),
                invalid.as_os_str(),
            ],
            "",
        );
        status(&output, 2);
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read_to_string(valid).unwrap(), "let x=1");
        assert_eq!(fs::read_to_string(invalid).unwrap(), source);
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
                        ..revofmt::FormatOptions::default()
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
    for option in [
        "--write",
        "--check",
        "--indent-width",
        "--indent-style",
        "--line-width",
        "--max-blank-lines",
        "--stdin-filepath",
        "--prefer-config",
        "--no-config",
        "--",
    ] {
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
fn indent_style_and_blank_line_flags() {
    let output = run(&["--indent-style", "tab"], "do\nfoo()\nend");
    status(&output, 0);
    assert_eq!(output.stdout, b"do\n\tfoo()\nend\n");
    let output = run(&["--max-blank-lines", "0"], "let a=1\n\n\nlet b=2");
    status(&output, 0);
    assert_eq!(output.stdout, b"let a = 1\nlet b = 2\n");
    let output = run(
        &["--max-blank-lines", "2", "--indent-style", "space"],
        "let a=1\n\n\n\nlet b=2",
    );
    status(&output, 0);
    assert_eq!(output.stdout, b"let a = 1\n\n\nlet b = 2\n");
}

#[test]
fn layout_flag_errors_name_the_flag() {
    for (args, message) in [
        (
            vec!["--indent-style"],
            "--indent-style requires space or tab",
        ),
        (
            vec!["--indent-style", "tabs"],
            "--indent-style requires space or tab",
        ),
        (
            vec!["--max-blank-lines"],
            "--max-blank-lines requires a non-negative integer",
        ),
        (
            vec!["--max-blank-lines", "-1"],
            "--max-blank-lines requires a non-negative integer",
        ),
        (
            vec!["--max-blank-lines", "many"],
            "--max-blank-lines requires a non-negative integer",
        ),
    ] {
        let output = run(&args, "let x=1");
        status(&output, 2);
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{args:?}"
        );
    }
    let output = run(&["--max-blank-lines", "9"], "let x=1");
    status(&output, 2);
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn rejects_invalid_arguments_without_source() {
    for args in [
        vec!["--check", "--write", "file.rv"],
        vec!["--indent-width"],
        vec!["--line-width"],
        vec!["--indent-width", "0"],
        vec!["--indent-width", "9"],
        vec!["--indent-width", "9", "--help"],
        vec!["--line-width", "19"],
        vec!["--line-width", "241"],
        vec!["--line-width", "241", "--version"],
        vec!["--line-width", "invalid"],
        vec!["--indent-style"],
        vec!["--indent-style", "tabs"],
        vec!["--max-blank-lines"],
        vec!["--max-blank-lines", "9"],
        vec!["--max-blank-lines", "-1"],
        vec!["--prefer-config", "--indent-width", "0"],
        vec!["--no-config", "--line-width", "19"],
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

const TAB_CONFIG: &str = "indent_style = \"tab\"\nline_width = 24\n";
const CALL_SOURCE: &str = "do\nconsume(first_argument, second)\nend";
const BLANK_LINES_SOURCE: &str = "let a=1\n\n\nlet b=2";

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn configuration_precedence_rows() {
    let dir = TempDir::new();
    dir.file("revofmt.toml", TAB_CONFIG);
    dir.file("a.rv", CALL_SOURCE);
    dir.file("b.rv", BLANK_LINES_SOURCE);
    let narrow_tabs = "do\n\tconsume(\n\t\tfirst_argument,\n\t\tsecond\n\t)\nend\n";
    let wide_tabs = "do\n\tconsume(first_argument, second)\nend\n";
    let wide_spaces = "do\n  consume(first_argument, second)\nend\n";
    for (flags, expected) in [
        (vec![], narrow_tabs),
        (vec!["--line-width", "80"], wide_tabs),
        (
            vec![
                "--prefer-config",
                "--line-width",
                "80",
                "--indent-style",
                "space",
            ],
            narrow_tabs,
        ),
        (vec!["--no-config"], wide_spaces),
        (
            vec![
                "--no-config",
                "--prefer-config",
                "--indent-style",
                "tab",
                "--line-width",
                "80",
            ],
            wide_tabs,
        ),
    ] {
        let mut args = flags.clone();
        args.push("a.rv");
        let output = run_in(&dir.0, &args, "");
        status(&output, 0);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            expected,
            "{flags:?}"
        );
        assert!(output.stderr.is_empty(), "{flags:?}");
    }
    // Under --prefer-config the file does not set max_blank_lines, so the
    // built-in default fills it rather than the flag.
    let output = run_in(
        &dir.0,
        &["--prefer-config", "--max-blank-lines", "0", "b.rv"],
        "",
    );
    status(&output, 0);
    assert_eq!(output.stdout, b"let a = 1\n\nlet b = 2\n");
    let output = run_in(&dir.0, &["--max-blank-lines", "0", "b.rv"], "");
    status(&output, 0);
    assert_eq!(output.stdout, b"let a = 1\nlet b = 2\n");
}

#[test]
fn stdin_filepath_discovers_from_nonexistent_and_relative_paths() {
    let dir = TempDir::new();
    dir.file("revofmt.toml", TAB_CONFIG);
    let source = "do\nfoo()\nend";
    let tabbed = b"do\n\tfoo()\nend\n";
    let missing = dir.0.join("missing").join("new.rv");
    let output = invoke(
        &[OsStr::new("--stdin-filepath"), missing.as_os_str()],
        source,
    );
    status(&output, 0);
    assert_eq!(output.stdout, tabbed);

    let output = run_in(&dir.0, &["--stdin-filepath", "sub/x.rv"], source);
    status(&output, 0);
    assert_eq!(output.stdout, tabbed);

    let present = dir.0.join("x.rv");
    let args = [
        OsStr::new("--check"),
        OsStr::new("--stdin-filepath"),
        present.as_os_str(),
    ];
    let output = invoke(&args, "do\n\tfoo()\nend\n");
    status(&output, 0);
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    status(&invoke(&args, source), 1);

    // Without the flag stdin performs no discovery, whatever the directory.
    let output = run_in(&dir.0, &[], source);
    status(&output, 0);
    assert_eq!(output.stdout, b"do\n  foo()\nend\n");
}

#[test]
fn stdin_filepath_requires_stdin() {
    for args in [
        vec!["--stdin-filepath", "x.rv", "a.rv"],
        vec!["--stdin-filepath", "x.rv", "-", "a.rv"],
        vec!["--check", "--stdin-filepath", "x.rv", "a.rv"],
        vec!["--stdin-filepath"],
    ] {
        let output = run(&args, "");
        status(&output, 2);
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
    }
    assert!(stderr(&run(&["--stdin-filepath"], "")).contains("--stdin-filepath requires a path"));
    assert!(
        stderr(&run(&["--stdin-filepath", "x.rv", "a.rv"], ""))
            .contains("--stdin-filepath requires stdin input")
    );
    // An explicit stdin marker is still stdin.
    status(&run(&["--stdin-filepath", "x.rv", "-"], "let x=1"), 0);
}

#[test]
fn stdin_configuration_errors_name_stdin_and_the_configuration() {
    let dir = TempDir::new();
    let config = dir.file("revofmt.toml", "indnet_width = 4\n");
    let anchor = dir.0.join("x.rv");
    let output = invoke(
        &[OsStr::new("--stdin-filepath"), anchor.as_os_str()],
        "let x=1",
    );
    status(&output, 2);
    assert!(output.stdout.is_empty());
    contains_path(&output, &config);
    assert!(stderr(&output).contains("stdin"));
    // --no-config never reads the malformed file.
    let output = invoke(
        &[
            OsStr::new("--no-config"),
            OsStr::new("--stdin-filepath"),
            anchor.as_os_str(),
        ],
        "let x=1",
    );
    status(&output, 0);
    assert_eq!(output.stdout, b"let x = 1\n");
}

#[test]
fn batches_resolve_each_input_separately() {
    let dir = TempDir::new();
    dir.file("one/revofmt.toml", "indent_style = \"tab\"\n");
    dir.file("two/revofmt.toml", "max_blank_lines = 0\n");
    let first = dir.file("one/a.rv", "do\nfoo()\nend");
    let second = dir.file("two/b.rv", BLANK_LINES_SOURCE);
    let output = invoke(
        &[OsStr::new("--write"), first.as_os_str(), second.as_os_str()],
        "",
    );
    status(&output, 0);
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(&first).unwrap(), b"do\n\tfoo()\nend\n");
    assert_eq!(fs::read(&second).unwrap(), b"let a = 1\nlet b = 2\n");
    let output = invoke(
        &[OsStr::new("--check"), first.as_os_str(), second.as_os_str()],
        "",
    );
    status(&output, 0);
}

#[test]
fn malformed_configuration_prevents_every_write() {
    let dir = TempDir::new();
    let good = dir.file("ok/a.rv", "let x=1");
    let config = dir.file("bad/revofmt.toml", "indnet_width = 4\n");
    let bad = dir.file("bad/b.rv", "let y=2");
    let output = invoke(
        &[OsStr::new("--write"), good.as_os_str(), bad.as_os_str()],
        "",
    );
    status(&output, 2);
    assert!(output.stdout.is_empty());
    contains_path(&output, &config);
    contains_path(&output, &bad);
    assert_eq!(fs::read(&good).unwrap(), b"let x=1");
    assert_eq!(fs::read(&bad).unwrap(), b"let y=2");
    // The reverse order, with the malformed input first, fails identically.
    let output = invoke(
        &[OsStr::new("--write"), bad.as_os_str(), good.as_os_str()],
        "",
    );
    status(&output, 2);
    assert_eq!(fs::read(&good).unwrap(), b"let x=1");
    // Print mode reports the error without output.
    let output = invoke(&[bad.as_os_str()], "");
    status(&output, 2);
    assert!(output.stdout.is_empty());
    contains_path(&output, &config);
}

#[test]
fn check_continues_past_configuration_errors() {
    let dir = TempDir::new();
    let good = dir.file("ok/a.rv", "let x=1");
    let config = dir.file("bad/revofmt.toml", "indnet_width = 4\n");
    let bad = dir.file("bad/b.rv", "let y = 2\n");
    let output = invoke(
        &[OsStr::new("--check"), bad.as_os_str(), good.as_os_str()],
        "",
    );
    status(&output, 2);
    assert!(output.stdout.is_empty());
    contains_path(&output, &config);
    let diagnostics = stderr(&output);
    assert!(
        diagnostics.contains(&format!("{}: requires formatting", good.display())),
        "{diagnostics}"
    );
    assert_eq!(fs::read(&good).unwrap(), b"let x=1");
}

#[test]
fn relative_inputs_resolve_against_the_working_directory() {
    let dir = TempDir::new();
    dir.file("revofmt.toml", "indent_style = \"tab\"\n");
    dir.file("a.rv", "do\n\tfoo()\nend\n");
    let output = run_in(&dir.0, &["--check", "a.rv"], "");
    status(&output, 0);
    assert!(output.stderr.is_empty());
    let output = run_in(&dir.0, &["--check", "--no-config", "a.rv"], "");
    status(&output, 1);
    // A nested working directory finds the file in its parent.
    dir.file("nested/b.rv", "do\nfoo()\nend");
    let output = run_in(&dir.0.join("nested"), &["b.rv"], "");
    status(&output, 0);
    assert_eq!(output.stdout, b"do\n\tfoo()\nend\n");
}

#[test]
fn overridden_invalid_config_values() {
    let dir = TempDir::new();
    let config = dir.file("revofmt.toml", "indent_width = 20\n");
    let source = dir.file("a.rv", "let x = 1\n");
    let path = source.as_os_str();
    let output = invoke(&[OsStr::new("--indent-width"), OsStr::new("2"), path], "");
    status(&output, 0);
    assert_eq!(output.stdout, b"let x = 1\n");
    let output = invoke(&[path], "");
    status(&output, 2);
    assert!(output.stdout.is_empty());
    contains_path(&output, &config);
    let output = invoke(
        &[
            OsStr::new("--prefer-config"),
            OsStr::new("--indent-width"),
            OsStr::new("2"),
            path,
        ],
        "",
    );
    status(&output, 2);
    assert!(output.stdout.is_empty());
    contains_path(&output, &config);
}

#[cfg(unix)]
#[test]
fn symlinked_directories_use_the_canonical_location() {
    let dir = TempDir::new();
    dir.file("project/revofmt.toml", "indent_style = \"tab\"\n");
    dir.file("outside/a.rv", "do\nfoo()\nend");
    let link = dir.0.join("project").join("link");
    std::os::unix::fs::symlink(dir.0.join("outside"), &link).unwrap();
    let through_link = link.join("a.rv");
    let output = invoke(&[through_link.as_os_str()], "");
    status(&output, 0);
    assert_eq!(output.stdout, b"do\n  foo()\nend\n");
    // The real location sits below no configuration, and the link is only a path.
    let direct = dir.0.join("outside").join("a.rv");
    let output = invoke(&[direct.as_os_str()], "");
    status(&output, 0);
    assert_eq!(output.stdout, b"do\n  foo()\nend\n");
}
