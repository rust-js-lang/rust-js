// The crates rust-js ships, each once (ADR 0102): its folder, its library's
// name, a program's `--extern`, and its Cargo and npm packages, in the order
// they're built, what one uses before it. A binding of what JS has, which
// writes no JS of its own, is compiled by rustc itself for its metadata
// (ADR 0112): src/cargo.rs's `BINDINGS`, which host-boundaries.test.ts
// holds to this, as it does each one to its Cargo.toml.
export const crates = [
  { dir: "builtins", lib: "js", cargo: "rust-js-builtins", npm: "@rust-js/builtins", binding: true },
  { dir: "webapi", lib: "webapi", cargo: "rust-js-webapi", npm: "@rust-js/webapi", binding: true },
  { dir: "react", lib: "react", cargo: "rust-js-react", npm: "@rust-js/react", binding: true },
  { dir: "node", lib: "node", cargo: "rust-js-node", npm: "@rust-js/node", binding: false },
  { dir: "next", lib: "next", cargo: "rust-js-next", npm: "@rust-js/next", binding: false },
];

// What a React program is compiled against: `react/build.sh` writes the
// react crate's metadata and, beside it, the crates it uses, each a
// program's to name (ADR 0102), `use js::spawn`.
export const reactLibs = ["react", "webapi", "js"];
