// Mutations of src/clippy.rs (ADR 0093).
import type { Mutation } from "../mutations";

const linted = ["test/clippy.test.ts", "-t", "rust-js clippy says what clippy finds"];

export const mutations: Mutation[] = [
  {
    name: "clippy-lints-unregistered",
    breaks: "`rust-js clippy` registers none of clippy's lints, and finds nothing of the Rust, a collapsible `if` say",
    file: "src/clippy.rs",
    find: "        clippy_lints::register_lint_passes(store, clippy_config::Conf::load(sess));\n",
    replace: "        let _ = clippy_config::Conf::load(sess);\n",
    tests: linted,
  },
  {
    name: "clippy-flags-untracked",
    breaks: "Cargo isn't told a crate's lint flags were read, and has it as checked when they change: `-D warnings` after a check without is no error",
    file: "src/clippy.rs",
    find: "        read.insert((\n            Symbol::intern(crate::cargo::CLIPPY),\n            value.as_deref().map(Symbol::intern),\n        ));\n",
    replace: "        let _ = &value;\n",
    tests: linted,
  },
];
