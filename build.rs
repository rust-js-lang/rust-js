//! Bake the toolchain's `lib` directory into the binary's rpath, so
//! `target/debug/rust-js` can find `librustc_driver` without extra env vars;
//! and the build's identity, `RUST_JS_BUILD`, a hash of what the compiler is
//! made of, which its manifests and Cargo are told (ADR 1003).

use std::path::{Path, PathBuf};

// The rpath to rustc's libraries, the one rule of it, the lowering's
// crates' build script too (#2299).
#[path = "rustc-rpath.rs"]
mod rustc_rpath;

fn main() {
    rustc_rpath::main();
    // What the compiler is made of: its sources and its crates' (ADR 1181),
    // which include its runtime helpers, the std manifest and the
    // toolchain's file it builds in, this script, and its package's
    // dependencies. Only those: without saying so,
    // Cargo runs it again, and builds rust-js again, when any file in the
    // package changes, a test or a doc too.
    let package = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("Cargo's package directory"));
    let root = package.join(file!()).parent().expect("the checkout").to_path_buf();
    let inputs = [
        root.join("src"),
        root.join("crates"),
        root.join("std/manifest.json"),
        root.join("std/compiled-std.json"),
        root.join("rust-toolchain.toml"),
        root.join("build.rs"),
        root.join("rustc-rpath.rs"),
        package.join("Cargo.toml"),
        package.join("Cargo.lock"),
    ];
    let mut files = Vec::new();
    for input in &inputs {
        println!("cargo:rerun-if-changed={}", input.display());
        collect(input, &mut files);
    }
    files.sort();
    // FNV-1a, of each file's path from the checkout and its bytes.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut add = |bytes: &[u8]| {
        for &byte in bytes {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    };
    for file in &files {
        let name = file.strip_prefix(&root).unwrap_or(file);
        add(name.to_string_lossy().as_bytes());
        add(&[0]);
        let bytes = std::fs::read(file).expect("read a compiler input");
        add(&(bytes.len() as u64).to_le_bytes());
        add(&bytes);
    }
    println!("cargo:rustc-env=RUST_JS_BUILD={hash:016x}");
    println!("cargo:rerun-if-env-changed=RUSTC");
}

/// Each file at `path`, a file or a directory's, recursively.
fn collect(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).expect("read a compiler input's directory") {
            collect(&entry.expect("a directory entry").path(), files);
        }
    } else if path.is_file() {
        files.push(path.to_path_buf());
    }
}
