# 0391. What works in Node only is one list, and panics in a browser

Status: Accepted. Extends [0272](0272-node.md) and
[0262](0262-when-rust-and-js-disagree.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)) in Node, whose process
is Rust's: its arguments and its exit, as native Rust's. D in a browser,
which has none: a panic that names what needed it, never silent drift.

## Context

std asks things of the process a program runs in: `env::args()`, its
arguments, `env::var(key)` and `env::vars()`, its environment, and
`process::exit(code)`, its end. Node has a process; a
browser hasn't. rust-js refused both, and with them rustc's tests of
arguments and of `exit`. Node's own modules are the `node` crate (ADR
0272): a program that imports one says it runs in Node. std's functions
say nothing of where they run, and one crate, a Next.js app's, is compiled
once for both its server and the browser.

## Decision

**std's functions that work in Node only are one list, recognition's
`NODE_ONLY`, each a helper of the runtime's one file of them,
`process.js`, which reads Node's `process` through `$process(what)`: in a
browser, that panics, naming what needed it.**

| Rust | JS |
|---|---|
| `env::args()` | `$args()`: `process.argv.slice(1)`, the program's path, then its arguments, as `node app.js a b` gives them; an array |
| `env::var(key)` | `$var(key)`: `Ok` of `process.env[key]`, or `Err` of `VarError::NotPresent`, `"NotPresent"` as an enum's unit variant is |
| `env::vars()` | `$vars()`: `Object.entries(process.env)`, an array of `[key, value]` |
| `process::exit(code)` | `$exit(code)`: `process.exit(code)` |

```js
const args = $args();
if (args.length > 3) {
  $exit(2);
}
```

In a browser:

```
Error: `std::env::args` needs Node's process, which a browser hasn't (ADR 0391)
```

- **One list, one file, one gate**: a function of Node's process is
  added to `NODE_ONLY`, as a `ProcessOp` whose `helper` is in
  `process.js`, which begins it with `$process`. `test/process.test.ts`
  holds the three equal, and that only `process.js`, and printing, which
  writes to `console` where there's no `process`, read `globalThis.process`.
- **Decided where it runs, not where it's compiled**: a Next.js app's one
  crate is its server's and its pages'. A call in `getStaticProps` works;
  the same in a component panics in the browser, as the call would in
  TypeScript, whose `process` is undefined there.
- **`env::args()` and `env::vars()` are arrays**, so `collect()` is
  each, and `env::args().len()` its `length`; so is any `v.iter().len()`
  now, not `$iter(v).end - $iter(v).at`.
- **A `VarError` is shown as std shows it**, "environment variable not
  found" and `NotPresent`. JS's environment is text, so `NotUnicode` is
  only one a program makes, shown as std shows it too.
- **The harnesses give a program native's arguments**: `corpus-run.ts`
  takes them after the outcome file, the corpus's `[case.js,
  outcome.json]` as its native binary's `[native, outcome.json]`, the
  rustc suite's the native binary's path alone; and a `process::exit`
  is how the program ended, `{ exit }`, which the rustc suite takes for
  native's exit of 0.

## Why

- **It's the JS a person writes in Node**, `process.argv`, `process.exit`.
- **In a browser it says why**: not JS's own `TypeError` of an undefined
  `process`, but which of std's functions needed Node.
- **It's tested**: the corpus's `env_args.rs` collects, counts, skips,
  steps and gives `env::args()`; `env_vars.rs` reads a variable there and
  one missing, shows a `VarError` of each kind, and finds, collects and
  steps `env::vars()`; `process.test.ts` exits with a code, and
  panics so in a browser; four rustc tests pass.

## Alternatives

- **A compile error where a crate says it runs in a browser**, a setting
  of where it runs: a Next.js app's crate runs in both, so the setting
  would be "both", and the error never made; and one more setting.
- **A JS stand-in in a browser**, `[]` of arguments: silent drift, a
  program that reads none where Rust's has its path.

## Consequences

- What's next of Node's, `env::current_dir` say, is one more `NODE_ONLY`
  entry and helper.
