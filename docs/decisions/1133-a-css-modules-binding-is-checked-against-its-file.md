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
  offers; its parser, printer and CSS Modules only.
- **The keys are what every bundler exports** (#1621), as the parity
  fixture finds them, `test/css-modules-parity.test.ts`: each construct a
  file of its own, built by Next 16.4's Turbopack, Next's webpack
  (css-loader) and Vite 8, each out of the box, `asIs`, their keys against
  rust-js's. They agree on classes, ids and keyframes (defined, or only
  named by `animation`), whatever holds them: nesting, `@media`,
  `@supports`, `@layer`, `@container`, `:global(..)` beside a local,
  `composes`, escapes and non-ASCII names. They disagree on custom
  identifiers: a view transition's or a counter style's name, which
  Turbopack alone exports. No bundler exports a grid's or a container's
  names, which Lightning CSS's default does. So the config is `grid`,
  `container` and `custom_idents` off (`container` is moot without
  `custom_idents`, kept as Next sets it), and a list's named counter
  style, which Lightning CSS exports whatever `custom_idents` says, is
  made a predefined one before the keys are read. A second list, with
  `custom_idents` on, is Turbopack's: a getter of a key only it has is
  refused naming who exports it, "exported by Turbopack, but not by
  webpack's css-loader or Vite", saying to read it by `"get []"`. Case
  ([0262](0262-when-rust-and-js-disagree.md)): results change, a class
  read `undefined` built by the others, so it's refused. Next's webpack
  also refuses a rule of no local class, `:global(.a) {}`, and `:local(.a)`
  alone, its CSS Modules pure; their keys agree where it builds. It adds
  `__checksum` to a module a server component imports, its own, no
  class of the file's.
- **Refused, never listed short** (#1617): ICSS's `:export { .. }` and
  `:import(..)`, which Lightning CSS reads as unknown pseudo-classes and
  exports nothing of, where css-loader exports `:export`'s names; and
  `@value`, which it refuses. A file it can't read is an error naming
  its line and column, counted from 1.
- **Size**: the native compiler (the dev profile it ships from) grew from
  60.12 MB to 65.58 MB, 9.1%. The WASI compiler, the playground's, built
  `--release --locked` at the toolchain's rustc, from 77.28 MB to 80.57 MB,
  4.3% (#1512).
- **Builds**: 53 crates more in a clean build, on both hosts; it builds
  and runs for `wasm32-wasip1`, read under Node's `node:wasi`
  ([#1512](https://github.com/rust-js-lang/rust-js/issues/1512#issuecomment-6841083967)).

## Consequences

- A misspelled class is rustc's error where it's read, and rust-js's at
  the binding where it's declared; plain `cargo check` passes, and
  rust-analyzer completes the getters.
- The struct lists the classes the program reads, not every one the file
  has: nothing observable depends on the rest.
- **The browser's compiler** is the same code: it reads the file from
  its virtual file system, as the playground gives it the crate's files,
  and where the file isn't there it's the same error naming it.
  `test/playground.test.ts` runs rust-js.wasm on such a crate in check's
  `wasm` job; `compile-rust.ts`' `runRustJs` is the one way it's run on a
  crate, the playground's own and a test's. Running such a program in the
  playground's page, which would need its bundler's CSS Modules, isn't
  part of this.
- **`rust-js bindings ./site.module.css`** prints the binding of every
  class the file exports, once, for the app to keep and trim: the form the
  check reads, as rustfmt writes it (`css_modules::binding`, the compiler's
  `--css-module-binding`, which the command runs). Each getter's name is
  the class's words, snake case, a keyword's raw, `r#type`. A class no
  Rust name says, `10`, `self`, or two that would share one, gets no
  getter but a note naming it: the person names it, or reads it by `get
  []`. Run where the crate root's JS is, it takes the path as a binding
  names it, so what it prints is what the check finds. It's never run by
  a build: the check keeps what it printed right, and nothing regenerates
  it.
- The parity fixture runs with the suite, some 6 s: three builds of 32
  small files. A bundler's upgrade that changes what it exports fails it,
  and the keys are decided again. 3c66a534b to this change used Lightning
  CSS's default config, which accepted a getter of a key no bundler, or
  only Turbopack, exports (#1621, P0).
- A bundler configured otherwise, css-loader's `exportLocalsConvention`
  or Vite's `css.modules`, isn't seen: the keys are each one's out of
  the box.
