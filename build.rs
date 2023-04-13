use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // The most reliable way is to check for the thumb-mode feature in
    // CARGO_CFG_TARGET_FEATURE but this is only available on nightly. As a
    // fallback we just check if the target name starts with "thumb".
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target = env::var("TARGET").unwrap();
    if arch == "arm" && target.starts_with("thumb") {
        println!("cargo:rustc-cfg=is_thumb");
    }
}
