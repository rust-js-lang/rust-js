# 1142. The React Compiler net: every component and hook rust-js emits compiles

Status: Accepted (2026-11-14, the user's decision: "React Compiler
compatibility is important to us, and needs a harness so it can't
regress").

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this is a net, not
a difference. What it finds is filed with the `react-compiler` label.

## Context

React Compiler memoizes a component or a hook only where it understands
it. Where it doesn't, it skips the function, quietly, and the app loses
its memoization. An app turns it on as create-vite's `react-compiler`
template does, `babel({ presets: [reactCompilerPreset()] })` from
`@vitejs/plugin-react` ([0041](0041-react.md)). Consumers found rust-js's
JS skipped where hand-written React isn't: a listener named as a local in
it (#1702), a `serde_json::Value`'s write through a getter handle (#1703).
Nothing held it: `test/vite.test.ts` checks one component's memo cache in a
browser.

## Decision

**Per push, Babel alone** (`test/react-compiler.test.ts`, under 2 s):
babel-plugin-react-compiler, the one `reactCompilerPreset` resolves from
`@vitejs/plugin-react`'s own place (the test says so, and that its version
is the one `package.json` pins), with a logger, over:

- the guide's output fences, `js` and `jsx`, a fence of several modules
  taken apart at their `// src/x.js` headers; one that isn't a module, two
  imports to compare, is skipped, as `test/docs.test.ts` holds what the
  compiler makes;
- examples/' committed JS of React's;
- `test/react-compiler/`'s cases, compiled there: a component as the guide
  teaches one, each consumer finding's repro, and props changed in place.

Every event but a success and a diagnostic, `CompileError`, `CompileSkip`,
`PipelineError`, fails, but where `test/react-compiler-known.txt` lists
its source, function and reason, with why, one of exactly two kinds:

- an open `react-compiler` issue, `#1702`, which `gh` confirms open and
  labeled where it can say; or
- `as hand-written: <file>.jsx`, JS a person would write under
  `test/react-compiler/hand-written/`, which fails the same, so the
  failure is React Compiler's or the program's, not rust-js's: a write to
  what a component's given as it renders, a ref read as it renders, a
  memo given a named function. The test runs each witness.

An entry nothing fails of fails too: take it off.

**Nightly, the lowering tests' programs** (`scripts/react-compiler-sweep.ts`,
two parts): each test file that compiles a program using React run with
its fixtures kept, and the same check over each module the compiler wrote,
beside its map. Its entries are `sweep:<fixture>`; run whole, one nothing
fails of fails. Many of its programs break React's rules to test
evaluation order, so those are `as hand-written`.

## Consequences

- Known today: #1702 (a promoted listener named as its local), and of the
  sweep #1729 (a BigInt literal in a component) and five as hand-written.
  #1705's repro as filed compiles, so it's no case until the consumer's
  shape is known; #1703 needs serde_json, its case with its fix.
- The mutation `react-compiler-net-catches-a-skip` leaves out the copy of
  props a component changes in place (#1723): its JS writes React's props,
  React Compiler refuses it, "This value cannot be modified", and the net
  fails.
- Making the net found #1723, a wrong answer: React keeps a component's
  props and gives them again, so rust-js's JS changing them in place
  showed 3 under `<StrictMode>` where Rust's gives 2.
