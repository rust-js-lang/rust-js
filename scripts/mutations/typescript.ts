// Mutations of src/typescript.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "printer-unnamed",
    breaks: "a build's `@rust-js/typescript`, `RUST_JS_TYPESCRIPT`'s, is no part of what Cargo is told rust-js is, so a change to it at the same path, a checkout's `typescript/index.js`, keeps the `.d.ts` the one before printed (#675)",
    file: "src/typescript.rs",
    find: "    let Some(module) = std::env::var_os(PRINTER) else {\n",
    replace: "    let Some(module) = std::env::var_os(PRINTER).filter(|_| false) else {\n",
    tests: ["test/cargo-workspace.test.ts", "-t", "told which declarations printer"],
  },
  {
    name: "printer-of-its-package-alone",
    breaks: "which `@rust-js/typescript` prints a build's `.d.ts` is its package's manifest alone, so a change to its module, a checkout's `typescript/index.js`, keeps the `.d.ts` the one before printed (#675)",
    file: "src/typescript.rs",
    find: "    for file in [module, package.as_path()] {\n",
    replace: "    for file in [package.as_path()] {\n",
    tests: ["test/cargo-workspace.test.ts", "-t", "told which declarations printer"],
  },
];
