// The crates rust-js ships, each once (ADR 0102): its folder, its library's
// name, a program's `--extern`, and its Cargo and npm packages, in the order
// they're built, what one uses before it. Each is a binding of what JS has,
// which writes no JS of its own, as its Cargo.toml says, `bindings = true`
// (ADR 0414), which host-boundaries.test.ts holds.
export const crates = [
  { dir: "builtins", lib: "js", cargo: "rust-js-builtins", npm: "@rust-js/builtins" },
  { dir: "webapi", lib: "webapi", cargo: "rust-js-webapi", npm: "@rust-js/webapi" },
  { dir: "react", lib: "react", cargo: "rust-js-react", npm: "@rust-js/react" },
  { dir: "node", lib: "node", cargo: "rust-js-node", npm: "@rust-js/node" },
  { dir: "next", lib: "next", cargo: "rust-js-next", npm: "@rust-js/next" },
];

// What a React program is compiled against: `react/build.sh` writes the
// react crate's metadata and, beside it, the crates it uses, each a
// program's to name (ADR 0102), `use js::spawn`.
export const reactLibs = ["react", "webapi", "js"];
