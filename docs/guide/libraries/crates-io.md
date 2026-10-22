# Crates from crates.io

A crate from crates.io, or from git, is a dependency as it is in any Rust
program: a line in the app's `Cargo.toml`. rust-js compiles it to JS with
the app, where it can, and the app's JS imports it
([ADR 0415](../../decisions/0415-a-crate-of-crates-io-in-an-app.md)).

```toml
[dependencies]
either = "=1.15.0"
```

```rust
use either::Either;

pub fn parse(text: &str) -> Either<u32, String> {
    match text.parse() {
        Ok(n) => Either::Left(n),
        Err(_) => Either::Right(text.to_string()),
    }
}

pub fn describe(text: &str) -> String {
    parse(text).either(|n| format!("number {n}"), |s| format!("text {s}"))
}
```

```js
import { Either } from "../rust-js/crates/either-1.15.0/src/lib.js";
import { $parseInt } from "@rust-js/runtime";

export function parse(text) {
  const parse = $parseInt(text, 0, 4294967295);
  if (parse.TAG === "Ok") {
    return { TAG: "Left", _0: parse._0 };
  }
  return { TAG: "Right", _0: text };
}

export function describe(text) {
  return Either.either(
    parse(text),
    (n) => `number ${n}`,
    (s) => `text ${s}`,
  );
}
```

## How they compile

`rust-js build` runs Cargo, which resolves the app's crates as it always
does, with rust-js as its `RUSTC_WRAPPER`
([ADR 0101](../../decisions/0101-cargo-workspace-wrapper.md)). Then:

- **Each crate from crates.io or git is tried,** in a process of its own.
  Compiled, its JS is the app's, in `rust-js/crates/<name>-<version>/`,
  beside the app's own: `rust-js/crates/either-1.18.0/src/lib.js`.
- **Commit it, as the app's own JS is.** `rust-js build --check` compares
  it, so a CI build needs no compile, and upgrading a crate is a diff to
  read: the new version's directory is written, and the old one's goes.
- **Only what the app imports is written:** a module of a crate is
  written where a module of the app's imports it, directly or through
  another.
- **A build script and a procedural macro are rustc's,** built and run on
  your machine as Cargo runs them. A derive's code is the crate's that
  uses it, compiled with it: thiserror's `Error` and strum's
  `EnumIter` are the app's JS.
- **A crate's lints are Cargo's to cap**, as for any registry crate: its
  warnings aren't the app's.

## What's refused

A crate rust-js can't compile is still checked by rustc, its types alone:
the app can name its types, and a call into it is refused, saying why the
crate couldn't be compiled. Here `crossbeam-utils` can't be, and its
note says what in it stopped it
([the limitations](../limitations.md)):

```toml
[dependencies]
crossbeam-utils = "=0.8.21"
```

```rust
pub fn doubled(n: u32) -> u32 {
    crossbeam_utils::thread::scope(|s| s.spawn(|_| n * 2).join().unwrap()).unwrap()
}
```

```
cargo check failed:
error: rust-js does not support calling `crossbeam_utils::thread::Scope::<'env>::spawn` yet
 --> src/lib.rs:2:42
  |
2 |     crossbeam_utils::thread::scope(|s| s.spawn(|_| n * 2).join().unwrap()).unwrap()
  |                                          ^^^^^^^^^^^^^^^^
  |
  = note: its crate is its types alone: rust-js couldn't compile it (ADR 0415): rust-js does not support statics of type `[cache_padded::CachePadded<atomic::seq_lock::SeqLock>; LEN]` yet

error: could not compile `app` (lib) due to 1 previous error
```

What stops a crate is what [the limitations](../limitations.md) list,
most often its raw memory: pointers read as another type, or into what
JS holds as a copy.

## Crates rust-js knows

A few crates are read as rustc builds them, never compiled by rust-js,
and rust-js writes their API as JS's own
([ADR 0423](../../decisions/0423-hashbrowns-maps-are-stds.md),
[ADR 0424](../../decisions/0424-indexmaps-maps-are-js-maps.md),
[ADR 0529](../../decisions/0529-a-shipped-memchr-searches-bytes.md)):

| Crate | In JS |
|---|---|
| `serde`, `serde_core`, `serde_json` | what a derive means, read from its attributes, and the runtime's JSON ([serde](README.md#serde)) |
| `hashbrown` (with `foldhash`, `allocator-api2`) | its `HashMap` and `HashSet` are std's: a `Map` and a `Set` |
| `indexmap` | its `IndexMap` and `IndexSet` are a `Map` and a `Set`, its methods by place, `get_index`, `swap_remove`, the runtime's |
| `memchr` 2.8.3, crates.io's | its searches are the runtime's, over the byte arrays slices are, `memchr` being `indexOf` |

```toml
[dependencies]
indexmap = "=2.14.2"
memchr = "=2.8.3"
```

```rust
pub fn first_comma(text: &[u8]) -> Option<usize> {
    memchr::memchr(b',', text)
}

pub fn counts(words: &[&str]) -> indexmap::IndexMap<String, u32> {
    let mut counts = indexmap::IndexMap::new();
    for word in words {
        *counts.entry(word.to_string()).or_insert(0) += 1;
    }
    counts
}
```

```js
import { $memchr, $orInsert } from "@rust-js/runtime";

export function firstComma(text) {
  return $memchr(text, [44]);
}

export function counts(words) {
  const counts = new Map();
  for (const word of words) {
    const current = $orInsert(counts, word, 0);
    counts.set(word, (current + 1) >>> 0);
  }
  return counts;
}
```

What one of these has that rust-js doesn't write is refused where it's
called, `memchr::memmem::find` among them. A `memchr` of another version,
a fork, or one by path or git is its types alone, and a call into it says
which to depend on: `cargo update -p memchr --precise 2.8.3`.

## Which crates work

[The crate corpus](../../crate-corpus.md) compiles common crates, each
with every crate it depends on, and runs a probe of each, a library that
uses it as an app would, beside native Rust: it **runs** where the two
print the same. As last measured:

| Runs as native Rust | Compiles, not yet shown to run | Refused, or blocked by a crate it uses |
|---|---|---|
| strum (`derive`), either, indexmap, bitflags, smallvec, arrayvec, hashbrown, thiserror, uuid, rust_decimal, chrono (`alloc`), heck, base64, hex | itertools, semver, num-traits | anyhow, once_cell, url, regex, time (`alloc`) |

The corpus says what stops each that doesn't, and when it was measured.
A crate that stops on something Rust allows is a gap in rust-js, to
report as [the limitations](../limitations.md) say.
