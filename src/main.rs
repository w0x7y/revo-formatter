mod cli;
// Task 7 wires the resolver into the CLI and removes this allowance.
#[allow(dead_code)]
mod config;

fn main() -> std::process::ExitCode {
    match cli::run(std::env::args_os().skip(1)) {
        Ok(code) => std::process::ExitCode::from(code),
        Err(message) => {
            eprintln!("revofmt: {message}");
            std::process::ExitCode::from(2)
        }
    }
}
