use std::{env, path::PathBuf, process::Command};
fn main() {
    for input in ["bridge.zig", "bridge", "vendor/revo"] {
        println!("cargo:rerun-if-changed={input}");
    }
    println!("cargo:rerun-if-env-changed=ZIG");
    let host = env::var("HOST").unwrap();
    let target = env::var("TARGET").unwrap();
    assert!(
        host == "x86_64-unknown-linux-gnu" && target == host,
        "revofmt currently supports native x86_64-unknown-linux-gnu builds only (HOST={host}, TARGET={target}); cross-target builds need separate validation"
    );
    let zig = env::var_os("ZIG").unwrap_or_else(|| "zig".into());
    let version = Command::new(&zig).arg("version").output().expect(
        "Install Zig 0.17.0 locally and set ZIG to its executable; builds never download tools",
    );
    assert!(
        version.status.success() && String::from_utf8_lossy(&version.stdout).trim() == "0.17.0",
        "revofmt requires exact stable Zig 0.17.0; set ZIG to the pinned executable"
    );
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let status = Command::new(zig)
        .args([
            "build-lib",
            "bridge.zig",
            "-O",
            "ReleaseSafe",
            "-target",
            "x86_64-linux-gnu",
            "-mcpu",
            "baseline",
            "-fPIC",
            "-fcompiler-rt",
            "-lc",
            "--name",
            "revo_frontend",
        ])
        .arg(format!(
            "-femit-bin={}",
            out.join("librevo_frontend.a").display()
        ))
        .args(["--cache-dir"])
        .arg(out.join("zig-cache"))
        .status()
        .expect("could not run Zig frontend build");
    assert!(status.success(), "pinned Revo frontend build failed");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=revo_frontend");
}
