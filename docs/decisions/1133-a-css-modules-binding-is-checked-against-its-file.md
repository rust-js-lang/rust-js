# 1133. A CSS Module's binding is checked against its file

Status: Accepted (2026-11-13, #1512). The user decided "verify a written
binding" on #1512 (2026-11-12), over the driver generating getters; the
coordinator approved this design and the Lightning CSS dependency.
Amends [0378](0378-one-file-at-a-time.md), "a file that isn't code is
bound as Next.js types it", for a CSS Module.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. The
JS is what the same binding gives without the mark; the check only
refuses.

## Context

Next.js types a CSS Module `{ readonly [key: string]: string }`, so a
misspelled class is `undefined` where it's read, and React renders
`class="undefined"`. An app moving to Rust kept a hand-written binding
of its classes, a struct of getters, which nothing checked against the
file: cross-agent-message's website's `Styles`, kept right by hand.

Two designs were weighed on #1512. Getters the driver generates from the
file would fail plain `cargo check` wherever a class is read outside
`jsx!`, since only a compile that read the CSS declares them, and gives
no completion in rust-analyzer. A binding the program writes, checked by
each compile, keeps every one of those: the getters are real
declarations. The user chose the second.

## Decision

**The binding is the one binding form ([0373](0373-one-binding-form.md)),
marked.** A struct of a JS object, `pub struct Styles(PhantomData<JsObject>)`,
marked `#[rust_js::css_module]`, its classes getters, `#[rust_js::link_name =
"get codeBlock"]`, and the module's default import a `static` of it,
`#[link_name = "./site.module.css#default"]`. Nothing is generated, and
the JS is the binding's as it would be unmarked: `STYLES.codeBlock`,
`STYLES["card-title"]`.

**Each compile checks it** (`lower/bindings/css_module.rs`, before
anything is lowered). For each `static` of a marked struct:

- its `link_name` is a relative `.module.css` file's default export, else
  an error saying so, naming that form;
- the file's keys are what the program's own files answer
  (`program::JsModules::css_module_keys`): found as `reached` finds a
  binding's JS file, by the specifier relative to the crate root's JS
  where it's written, then where it goes in source ([0028](0028-js-module-imports.md));
- each getter's class, `get k` of `&self`, is one of them, else an error
  at its `link_name`, naming the file's classes. `"get []"`, a class named
  as the program runs ([1011](1011-a-bindings-index-signature-is-index.md)),
  isn't checked; any other form is an error saying what to write.

A `#[rust_js::css_module]` that isn't a struct is an error. A struct
marked but bound by no `static` is checked by none.

**The file is what the build read.** `LocalJs`, which reads the program's
own files for lowering, records it with the JS files bindings import, so
it's in the build's dep-info and Cargo's: a save of the stylesheet checks
the program again, and a class it deletes that the program reads is the
error. A save that changes no class compiles the same JS.

**One function knows CSS** (`css_modules::exported_keys`): text in, sorted
keys out. It's Lightning CSS's, as Turbopack's are, with its CSS Modules
config and nothing else, so it implements CSS Modules' own rules of what's
exported, `:local` and `:global`, `composes`, keyframe and grid names,
once. A second lister over `cssparser` alone was weighed (smaller), and
refused: a second implementation of those rules, which can miss a key.
Lightning CSS only on the native compiler was refused too: the native
and browser compilers would disagree.

- **Pinned**: `lightningcss = "=1.0.0-alpha.72"`, `default-features =
  false`: none of `bundler`, `nodejs`, `sourcemap`, `browserslist`, `visitor`,
  `into_owned`, `serde`, `jsonschema`, `substitute_variables` or `cli` the crate
  offers; its parser, printer and CSS Modules only. Its config is
  `css_modules::Config::default()`: classes, `@keyframes`, grid, container
  and custom identifiers scoped, `--dashed` ones not.
- **Refused, never listed short** (#1617): ICSS's `:export { .. }` and
  `:import(..)`, which Lightning CSS reads as unknown pseudo-classes and
  exports nothing of, where css-loader exports `:export`'s names; and
  `@value`, which it refuses. A file it can't read is an error naming
  its line and column, counted from 1.
- **Size**: the native compiler (the dev profile it ships from) grew from
  60.12 MB to 65.58 MB, 9.1%. The WASI compiler's growth is bounded by
  the probe's whole Lightning CSS, 3.6 MB of 73.3 MB, at most 4.9%; check's
  `wasm` job measures it on this change's first push (#1512).
- **Builds**: 53 crates more in a clean build, on both hosts; it builds
  and runs for `wasm32-wasip1`, read under Node's `node:wasi`
  ([#1512](https://github.com/rust-js-lang/rust-js/issues/1512#issuecomment-6841083967)).

## Consequences

- A misspelled class is rustc's error where it's read, and rust-js's at
  the binding where it's declared; plain `cargo check` passes, and
  rust-analyzer completes the getters.
- The struct lists the classes the program reads, not every one the file
  has: nothing observable depends on the rest.
- Still to come on #1512: the playground's compiler reading the file
  through its virtual file system, where it's absent a compile error naming
  it; `rust-js bindings ./x.module.css` printing the binding this checks,
  once; and the parity fixture, Turbopack, webpack's css-loader and Vite,
  `asIs`, which settles which constructs' keys count, ids, grid and
  container names among them, and refuses the rest. Until it lands the
  config is Lightning CSS's default, Turbopack's parser's.
