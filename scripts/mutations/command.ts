// Mutations of src/command.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-metadata-unstaged",
    breaks: "rustc writes a library's metadata where it's asked for, before its JS is published, and whether it is or not",
    file: "src/command.rs",
    find: "                        rewritten.push(format!(\"metadata={}\", to.display()));\n",
    replace: "                        rewritten.push(format!(\"metadata={}\", path.display()));\n                        std::fs::create_dir_all(dir()).ok();\n                        std::fs::write(&to, b\"\").ok();\n",
    tests: ["test/crates.test.ts", "-t", "neither is"],
  },
  {
    name: "rustc-outputs-passed-through",
    breaks: "rustc's other outputs, `--emit=mir` say, are written past rust-js's checks",
    file: "src/command.rs",
    find: "                    None => {\n                        return Err(format!(\n                            \"rustc's `--emit={kind}` isn't something rust-js writes; only a library's --emit=metadata=<path>\"\n                        ));\n                    }\n",
    replace: "                    None => rewritten.push(kind.to_string()),\n",
    tests: ["test/crates.test.ts", "-t", "outputs are refused"],
  },
  {
    name: "rustc-bootstrapped",
    breaks: "rust-js turns its rustc's unstable features on, as `RUSTC_BOOTSTRAP=1` does, so a crate's own `#![feature]` compiles where a stable release refuses it",
    file: "src/command.rs",
    find: "        let mut args = vec![\n            \"rust-js\".to_string(), // argv[0], ignored by rustc",
    replace: "        unsafe { std::env::set_var(\"RUSTC_BOOTSTRAP\", \"1\") };\n        let mut args = vec![\n            \"rust-js\".to_string(), // argv[0], ignored by rustc",
    tests: ["test/diagnostics.test.ts", "-t", "a crate's own #!\\[feature\\] is refused, as on a stable release"],
  },
  {
    name: "library-without-manifest",
    breaks: "a library is built without the manifest its JS is found by",
    file: "src/command.rs",
    find: "    if library && manifest.is_none() {\n",
    replace: "    if false && library && manifest.is_none() {\n",
    tests: ["test/manifest.test.ts", "-t", "refused without its manifest"],
  },
  {
    name: "rust-js-cfg-unset",
    breaks: "rust-js doesn't set `cfg(rust_js)`, and the binding crates' metadata leaves out every `#[rust_js::link_name]`",
    file: "src/command.rs",
    find: "pub const RUST_JS_CFG: [&str; 2] = [\"--cfg=rust_js\", \"--check-cfg=cfg(rust_js)\"];\n",
    replace: "pub const RUST_JS_CFG: [&str; 2] = [\"--check-cfg=cfg(rust_js)\", \"--check-cfg=cfg(rust_js)\"];\n",
    tests: ["test/snapshots.test.ts", "-t", "counter"],
  },
  {
    name: "flag-value-apart-unread",
    breaks: "a flag's value given apart, Cargo's `--out-dir target/..`, is no value, and `rust-js --rustc` can't find where Cargo writes",
    file: "src/command.rs",
    find: "            flags.get(i + 1).cloned()\n",
    replace: "            None\n",
    tests: ["test/cargo-react.test.ts"],
  },
  {
    name: "panic-runtime-not-given",
    breaks: "a test harness, which rustc gives a panic runtime, is given wasm32's, which aborts, and rustc refuses it",
    file: "src/command.rs",
    find: "            args.push(format!(\"--extern=panic_unwind={}\", runtime.display()));\n",
    replace: "",
    tests: ["test/manifest.test.ts", "-t", "where its tests' runner is"],
  },
];
