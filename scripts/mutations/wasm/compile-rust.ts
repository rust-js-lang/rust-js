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
  {
    name: "wasi-crate-files-emptied",
    breaks: "a crate's files given rust-js.wasm by their texts, a CSS Module's binding test's `lib.rs` and stylesheet, are written empty in its file system, so it compiles nothing of what it was given (#2346)",
    file: "wasm/web/compile-rust.ts",
    find: "  return new Map(Object.entries(texts).map(([name, text]) => [name, new File(new TextEncoder().encode(text))]));\n",
    replace: "  return new Map(Object.entries(texts).map(([name]) => [name, new File(new TextEncoder().encode(\"\"))]));\n",
    tests: ["test/playground.test.ts", "-t", "rust-js.wasm checks a CSS Module's binding against the file in its file system"],
    needs: "wasm",
  },
];
