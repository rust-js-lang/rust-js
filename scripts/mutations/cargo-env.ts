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
];
