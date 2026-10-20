# rust-js ships the crates that read bytes at addresses

Status: Draft. Would extend [0415](../0415-a-crate-of-crates-io-in-an-app.md),
whose crates of crates.io's are compiled where they can be, and
[0423](../0423-hashbrowns-maps-are-stds.md), whose hashbrown is read as
rustc builds it and its maps are std's; and [0407](../0407-a-raw-pointer-is-a-handle-on-its-place.md)
and [0482](../0482-a-slices-pointer-cast-to-the-same-place-is-the-slice.md),
which model the pointers JS can.

Case: A ([0262](../0262-when-rust-and-js-disagree.md)) for what each
shipped crate's API gives, held equal to native by its probe; D, an error,
for what a shipped crate's Rust does that its JS can't, named where it's
called.

## Context

JS has no addressable memory. rust-js models what it can, a pointer to a
place, into an array, a `transmute` of numbers, and refuses the rest
([the limitations](../../limitations.md)). That refusal is the one wall the
crate corpus can't get past by a compiler change: regex stops at memchr, a
pointer into what JS may hold a copy of; url at yoke, raw pointers that
erase lifetimes; time at powerfmt, a pointer to a slice of `MaybeUninit`s;
anyhow at a `*const dyn Error`; semver at a pointer's address. Forty of
rustc's hundred known failures are the same wall.

These crates have one thing in common: each is a byte engine, a SwissTable,
a SIMD scan, a DFA, a parser over bytes, whose JS, compiled as written,
would be the engine, not the JS a person writes, who has `RegExp`, `URL`
and `TypedArray`. hashbrown was exactly this, and ADR 0423 answered it:
rustc builds the crate, and its API is what the platform's equivalent is
in JS. Scala.js reached the JDK's coverage the same way, by writing its
own javalib over the platform, never by compiling the JDK.

The alternatives don't reach these crates:

- **Modelling pointer patterns one by one**, as ADRs 0407 and 0482 do,
  closes what's near a place: `*const dyn T` as the pair, a pointer's
  equality as identity. It never reaches a scan that reads eight bytes as
  a `u64`.
- **A linear memory**, a byte heap with values marshalled at its edge, is
  a second representation of every value, unreadable, and the end of the
  north star for every crate that touches it.
- **Excluding them** leaves regex and url, which every app reaches,
  outside the supported product (ROADMAP M1.3).

## Decision

**A crate whose Rust reads bytes at addresses, and whose API the platform
has, is read as rustc builds it and its API is rust-js's, written over the
platform's: regex over `RegExp`, memchr over `TypedArray`, url over `URL`.
What its JS can't do as its Rust does is refused where it's called, naming
the crate and the method.**

```rust
let re = Regex::new(r"(\d+)-(\d+)").unwrap();
let caps = re.captures("10-20").unwrap();
println!("{}", &caps[1]);
```

```js
const re = $regex("(\\d+)-(\\d+)");
const caps = re.exec("10-20");
console.log(`${caps[1]}`);
```

- **Which crates are shipped is one list**, `cargo::READ_AS_RUSTC_BUILDS_THEM`'s,
  where hashbrown already is, each with the crate that ships its JS:
  `regex` and `regex-automata` by `@rust-js/regex`, `memchr` by
  `@rust-js/memchr`, `url` by `@rust-js/url`. A shipped crate is rustc's
  wherever it is, a registry's or the app's own by its path (ADR 0423).
- **Which of its items are rust-js's is `recognition`'s to say by path**,
  as `bound_map_item` says hashbrown's: `regex::Regex`, `regex::Captures`,
  `url::Url`. An item it doesn't name, `regex::bytes::Regex`'s pointer
  into its haystack, is refused where it's called: rustc built the crate,
  so there's no JS of it.
- **Its JS is a binding crate, hand-written, as the js, webapi and react
  crates are** ([bindings by hand](../0118-bindings-on-npm-only.md)): a
  `#[rust_js::binding]` module whose Rust signatures are the crate's own,
  so an app's `use regex::Regex` resolves to the registry's types, as
  rustc checks them, and its calls to the binding's JS.
- **Each shipped crate's semantics are held equal to native by its
  probe**, `scripts/crate-corpus/<name>.rs`, run both ways (the crate
  corpus). Where the platform differs, the probe says which: a regex
  syntax `RegExp` lacks, `(?P<name>)`, `\p{Greek}` spelled differently,
  lookaround regex refuses, is translated where it's one rewrite and
  refused with the pattern where it isn't, at `Regex::new`, which in JS
  is where a pattern is compiled too. A `Url` is WHATWG's where it's the
  crate's, and `url`'s own methods the platform hasn't, `path_segments_mut`,
  are written over `URL`'s parts.
- **A shipped crate's dependents compile as before**: anyhow isn't
  shipped, its `*const dyn Error` is modelled or refused on its own; time
  isn't, powerfmt is: a shipped crate is the one at the wall, never the
  one above it.

## Why

- **It's what a person writes**: `new RegExp`, `new URL`, as a `Map` is
  what std's `HashMap` already is.
- **It's checked, both ways**: each probe is the test, as hashbrown's is;
  `test/registry-crates.test.ts` holds that a shipped crate's name is
  never compiled.
- **It reaches the wall**: regex, url and time compile, 18 of the corpus's
  22, and every app that parses a URL or a pattern.
- **It's the north star's own words**: "a crate rebound from scratch, if
  it serves the north star". Scala.js's javalib is the proof it scales.

## Alternatives

- **A linear memory for named crates only**: faithful to every byte, but
  two value representations and a marshalling layer at every call from
  the app, and the JS of regex would be regex's DFA. Rejected for
  readability and for the second way it adds.
- **Compiling regex-lite instead**, which avoids memchr: it still reads
  bytes through pointers, and an app's `Cargo.toml` says `regex`.
- **Generating the binding from the crate's rustdoc**: the north star
  keeps bindings by hand (ADR 0118); a generated one would be a second
  way to bind.

## Consequences

- **A list of crates rust-js must keep**, one binding crate each, versioned
  with the Rust crate's major: a maintenance commitment the crate corpus
  measures.
- **A regex that `RegExp` can't run is an error at `Regex::new`**, named
  in the limitations page under a new heading, Shipped crates, with the
  pattern that shows it.
- **rustc's known failures that read addresses stay refused**: they're
  programs written to exercise pointers, not libraries, and ROADMAP M1.3
  records them as excluded.
- **Order of work**: memchr first, its probe the scans regex uses; then
  regex, whose probe is regex's own doc examples; then url. time waits on
  powerfmt, which is small and may be modelled instead.
