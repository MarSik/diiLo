use std::process::Command;

use chrono::{Datelike, Local};

fn main() {
    let git_hash = if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        String::from_utf8(output.stdout).unwrap()
    } else {
        String::with_capacity(0)
    };
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=BUILD_YEAR={}", Local::now().year())
}
