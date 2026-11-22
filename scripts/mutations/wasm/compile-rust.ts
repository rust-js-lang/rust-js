// Mutations of wasm/web/compile-rust.ts (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "playground-wasi-root-named",
    breaks: "rust-js.wasm's compile of the playground names each file under the WASI file system's root, `/wasm/web/rust/compiler.rs`, where the native compiler names `wasm/web/rust/compiler.rs`, so their panics' locations differ (#2345)",
    file: "wasm/web/compile-rust.ts",
    find: 'export const playgroundRustc = ["--remap-path-prefix=/="];\n',
    replace: "export const playgroundRustc: string[] = [];\n",
    tests: ["test/playground.test.ts", "-t", "names its files as the native compiler does"],
  },
];
