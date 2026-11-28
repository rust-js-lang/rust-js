// Mutations of tooling/cargo.js: Cargo's environment of a build through
// rust-js (`cargoEnv`).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "compat-name-not-given-to-cargo",
    breaks: "Cargo's environment has no `RUST_JS_COMPAT`, so the compiler, its wrapper, refuses every compile (#2438)",
    file: "tooling/cargo.js",
    find: "    RUST_JS_COMPAT: compatNamed(),\n",
    replace: "",
    tests: ["test/cargo.test.ts", "-t", "rust-js's own crates' panics name their files the same on every machine"],
  },
  {
    name: "exit-awaited-alone",
    breaks: "a child's end waits for its `exit` alone, which Bun's `node:child_process` now and then never emits, its exit code set, so the builder's run of `rust-js --version-json` waits a push's check out (#2639)",
    file: "tooling/cargo.js",
    find: "    const asked = setInterval(ended, every);\n",
    replace: "    const asked = undefined;\n    void ended;\n",
    tests: ["test/cargo.test.ts", "-t", "a child's end is known by its exit code"],
  },
];
