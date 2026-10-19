use std::env;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=src/api/endpoints.rs");
    println!("cargo:rustc-check-cfg=cfg(license)");

    let source = fs::read_to_string("src/api/endpoints.rs").unwrap_or_default();
    let enabled = source
        .lines()
        .find(|line| line.contains("USE_LICENSE"))
        .and_then(|line| line.split('=').nth(1))
        .map(|value| value.trim().trim_end_matches(';').trim() == "true")
        .unwrap_or(false);

    if enabled {
        println!("cargo:rustc-cfg=license");
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or(0);

    println!("cargo:rustc-env=BUILD_TS={}", stamp);

    if let Ok(target_os) = env::var("CARGO_CFG_TARGET_OS") {
        if target_os == "windows" {
            let mut res = winres::WindowsResource::new();
            res.set_icon("assets/icon.ico");

            if let Err(e) = res.compile() {
                println!("cargo:warning=Failed to attach Windows icon: {}", e);
            }
        }
    }
}
