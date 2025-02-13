use std::env;
use std::path::PathBuf;

fn main() {
    // Build Interception
    let dst = cmake::build("libs/interception");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=interception");

    // Build ViGEm
    let dst = cmake::build("libs/vigem");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=vigem");

    // Ensure DLLs are copied to output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    std::fs::copy(
        "libs/interception/interception.dll",
        out_dir.join("interception.dll"),
    ).unwrap();
    std::fs::copy(
        "libs/vigem/ViGEmBus.dll",
        out_dir.join("ViGEmBus.dll"),
    ).unwrap();
} 