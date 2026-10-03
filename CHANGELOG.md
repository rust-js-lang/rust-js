# Changelog

Each release of rust-js's compiler and the packages released with it, at its
version: `@rust-js/native`, `@rust-js/build`, `@rust-js/vite-plugin`,
`@rust-js/runtime`, `@rust-js/resources`, `@rust-js/create` and
`@rust-js/react` ([ADR 0120](docs/decisions/0120-first-npm-release.md)).
`@rust-js/builtins` and `@rust-js/webapi` are released on their own, each
at its version ([ADR 0116](docs/decisions/0116-binding-versions.md)).

## 0.0.3

For macOS on Apple silicon, with Rust 1.98.1, as 0.0.2. `@rust-js/builtins`
stays 0.0.1 and `@rust-js/webapi` 0.0.2.

### New

rustc's own tests that rust-js compiles and runs as Rust does: 1,899 of
2,206, from 1,583. Among what compiles now:

- **Text by its bytes:** a string's `len()`, slices and `find` count UTF-8
  bytes, as Rust's do; `s.bytes()`, `as_bytes()` and byte strings.
- **Numbers:** `f32`, each result rounded as Rust rounds it; bitwise
  operators in generic code and a type's own, as a set of flags has them.
- **Patterns:** slice and array patterns, bindings inside `|`, `if let` as a
  value anywhere, if-let guards, inline `const` blocks.
- **Formatting:** `{:#?}` and the Debug builders; a placeholder's width,
  precision and sign reach a generic or `dyn` value's `fmt`, which can ask
  its `Formatter` for them.
- **Traits and generics:** const generics of traits, a `dyn Display` or
  `dyn Error` as a value, `Box<dyn Error>` from `?`, generic associated
  types of lifetimes, `size_of` and `type_name` of a type parameter.
- **Iterators:** std's sources, lazy `chain`, `zip`, `take_while`,
  `skip_while`, `fuse`; ranges as values; iterator trait objects; a generic
  iterator stepped through or lent as a `&mut`; an iterator's `len()`;
  `collect()` into a `Result` or an `Option`.
- **Shared state:** `Mutex`, `RwLock` and `Arc` on one thread, channels on
  one thread, a `HashMap` or `HashSet` keyed by a struct.
- **Destructors** run where Rust runs them for closures, loops, trait
  objects and temporaries too.

### Fixed: programs that gave a wrong answer

Each of these compiled in 0.0.2 and ran differently than Rust. A program that
did any of them prints Rust's answer now:

- `collect::<Result<Vec<_>, _>>()` and `collect::<Option<Vec<_>>>()` gave the
  array of `Result`s or `Option`s, not the first `Err` or `None`.
- `println!("{} {:?}", v.len(), v.pop())` read `v.len()` after the pop.
- `next()` of an iterator chain could take more than one item, and a chain
  whose closures do what can be seen, kept in a variable or given to `zip`
  or `chain`, could run them in another order than Rust's.
- A width or a sign for a generic or `dyn` value was dropped: it reaches the
  value's `fmt` now.

### May now be refused

What 0.0.2 compiled wrongly is now either right or an error that names it.
A program that relied on one of these stops at the error, where it ran
wrongly before: a width or a sign for a `&dyn Debug` made elsewhere, or for
serde_json's types, whose `fmt`s are rust-js's own.

### Upgrade from 0.0.2

1. In `package.json`, set each package released with the compiler to
   `0.0.3`: `@rust-js/runtime` and `@rust-js/react` in `dependencies`,
   `@rust-js/native`, `@rust-js/build` and `@rust-js/vite-plugin` in
   `devDependencies`. Leave `@rust-js/builtins` and `@rust-js/webapi`.
2. In `Cargo.toml`, name the react crate's release:
   `react = { package = "rust-js-react", version = "~0.0.3" }`.
3. `bun install` (or npm's, pnpm's): its `postinstall`, `rust-js-patch`,
   points Cargo at the new crates in `node_modules`.
4. `bun run dev` or `bun run build`: rust-js writes the JS beside your Rust
   again. Commit it with the upgrade: its diff is what the new compiler
   writes differently.

### Roll back to 0.0.2

The same steps, with `0.0.2` and `~0.0.2`: `bun install` again, then build,
and the JS beside your Rust is 0.0.2's again (`git checkout` of it does
the same).

## 0.0.2

An app is a Cargo package with its crates from npm, `@rust-js/react` among
them, first released here; its patch is written as it's installed and as
Vite starts, and a Cargo build runs the compiler's Rust. A release build
has no path of the building machine's home in it.

## 0.0.1

The first release on npm, for macOS on Apple silicon: `bun create @rust-js`
makes a Vite and React app written in Rust, with `@rust-js/builtins` and
`@rust-js/webapi`.
