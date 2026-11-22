//! The toolchain's `lib` directory in what links rustc's crates, its rpath,
//! so a binary finds `librustc_driver` with no environment set: the
//! compiler, `target/debug/rust-js`, and the unit test binaries of the
//! lowering's crates (#2299), which the mutation runner runs from a copy,
//! where Cargo's library path isn't set. The build script of those crates,
//! and a part of the compiler's own, build.rs, which takes this file as a
//! module and calls its `main`: the one rule of it. The browser's compiler,
//! `wasm/`'s, links rustc's crates it builds itself, nothing of the
//! toolchain's.

use std::process::Command;

/// The rpath, said to Cargo for what this package links.
pub fn main() {
    let family = std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    if family.split(',').any(|family| family == "wasm") {
        return;
    }
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let out = Command::new(rustc).arg("--print=sysroot").output().expect("run rustc");
    let sysroot = String::from_utf8(out.stdout).expect("utf-8 sysroot");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib", sysroot.trim());
}
