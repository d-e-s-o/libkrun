use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let libkrun_root = manifest_dir.join("../..").canonicalize().unwrap();
    let init_src = libkrun_root.join("init/init.c");
    let init_bin = libkrun_root.join("init/init");

    if !init_src.exists() {
        return;
    }

    println!("cargo:rerun-if-changed={}", init_src.display());
    println!(
        "cargo:rerun-if-changed={}",
        libkrun_root.join("init/jsmn.h").display()
    );

    if init_bin.exists() {
        return;
    }

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = Command::new(&cc)
        .args(["-O2", "-static", "-Wall", "-o"])
        .arg(&init_bin)
        .arg(&init_src)
        .status()
        .unwrap_or_else(|e| panic!("failed to execute {cc}: {e}"));

    if !status.success() {
        panic!("failed to compile init/init.c: {status}");
    }
}
