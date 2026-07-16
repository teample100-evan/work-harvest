use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| !output.stdout.is_empty());
    let built_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    println!("cargo:rustc-env=WORK_HARVEST_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=WORK_HARVEST_BUILD_DIRTY={dirty}");
    println!("cargo:rustc-env=WORK_HARVEST_BUILT_AT_UNIX={built_at_unix}");
    tauri_build::build()
}
