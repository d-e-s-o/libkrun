use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

fn default_init_src_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let dir = manifest_dir.join("../../init");
    dir
}

fn build_default_init() -> PathBuf {
    let init_root = default_init_src_dir();
    let init_src = init_root.join("init.c");
    let dhcp_src = init_root.join("dhcp.c");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let init_bin = out_dir.join("init");

    println!("cargo:rerun-if-env-changed=CC_LINUX");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=TIMESYNC");
    println!("cargo:rerun-if-changed={}", init_src.display());
    println!("cargo:rerun-if-changed={}", dhcp_src.display());
    println!(
        "cargo:rerun-if-changed={}",
        init_root.join("jsmn.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        init_root.join("dhcp.h").display()
    );

    let mut init_cc_flags = vec!["-O2", "-static", "-Wall"];
    if std::env::var_os("TIMESYNC").as_deref() == Some(OsStr::new("1")) {
        init_cc_flags.push("-D__TIMESYNC__");
    }

    let cc_value = std::env::var("CC_LINUX")
        .or_else(|_| std::env::var("CC"))
        .unwrap_or_else(|_| "cc".to_string());
    let mut cc_parts = cc_value.split_ascii_whitespace();
    let cc = cc_parts.next().expect("CC_LINUX/CC must not be empty");
    let status = Command::new(cc)
        .args(cc_parts)
        .args(&init_cc_flags)
        .arg("-o")
        .arg(&init_bin)
        .arg(&init_src)
        .arg(&dhcp_src)
        .status()
        .unwrap_or_else(|e| panic!("failed to execute {cc}: {e}"));

    if !status.success() {
        panic!("failed to compile init/init.c: {status}");
    }
    init_bin
}

fn main() {
    let init_binary_path = std::env::var_os("KRUN_INIT_BINARY_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if default_init_src_dir().exists() {
                let init_path = build_default_init();
                // SAFETY: The build script is single threaded.
                unsafe { std::env::set_var("KRUN_INIT_BINARY_PATH", &init_path) };
                init_path
            } else {
                // XXX: Fall back to an empty init file.
                PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("dummy")
            }
        });
    println!(
        "cargo:rustc-env=KRUN_INIT_BINARY_PATH={}",
        init_binary_path.display()
    );
    println!("cargo:rerun-if-env-changed=KRUN_INIT_BINARY_PATH");
}
