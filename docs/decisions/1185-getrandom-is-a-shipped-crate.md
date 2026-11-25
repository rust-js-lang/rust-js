# 1185. getrandom is a shipped crate: Web Crypto's bytes, its `Error` its code

Status: Accepted (2026-11-23, #2326; Tu approved worker-02 agent's design, that
getrandom go ADR 0529's memchr route; the lead approved the extension to
its `Error`, the version fact, the backend rule and keeping getrandom in
covered.txt). Extends [0529](0529-a-shipped-memchr-searches-bytes.md);
supersedes [1112](1112-wasm-bindgen-for-dependencies.md)'s getrandom
part: getrandom's `wasm_js` backend compiled through rust-js's
wasm-bindgen is no longer how getrandom runs.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
getrandom's own behavior is kept: each version's text for each code, and
for an OS's number the native unix build's `strerror` words, through ADR
0462's table. None for Web Crypto in place of each platform's entropy
source: random bytes are random bytes, and only the error path differs,
which no host with Web Crypto takes. Refused where Rust's can't be kept:
the rest of getrandom.

## Context

rust-js checks crates for `wasm32-unknown-unknown` (ADR 0090), and
getrandom has no entropy source there unless its user picks one.
getrandom 0.1 without its `wasm-bindgen` feature takes its backend that
always fails. 0.2 refuses to compile without its `js` or `custom` feature.
0.3 and 0.4 refuse without a `getrandom_backend` cfg or, in 0.4, the
`wasm_js` feature. rand 0.7's `thread_rng` panicked, for example, so 91 of
itertools' quickcheck tests and 58 of its other tests failed (#2326). ADR
1112 covered getrandom 0.3 with `wasm_js` and 0.1 with `wasm-bindgen`, by
compiling their backends through rust-js's own wasm-bindgen, but only
where the graph turned those features on, and never 0.2's `js`.

## Decision

**getrandom is a shipped crate, built by rustc. rust-js writes its API
as one runtime helper over `crypto.getRandomValues`, at each version the
catalog qualifies: 0.1.16, 0.2.17, 0.3.4 and 0.4.3, whatever features its
user chose. Its `Error` is its code, a number, shown as that version
shows it.**

| Rust | JS |
|---|---|
| `getrandom::getrandom(&mut buf)` (0.1, 0.2) | `$getrandom(buf, "0.2")` |
| `getrandom::fill(&mut buf)` (0.3, 0.4) | `$getrandom(buf, "0.3")` |
| `getrandom_uninit(&mut buf)`, `fill_uninit(&mut buf)` | `$getrandom(buf, "0.3", true)`, `Ok` of `buf` |
| `getrandom::u32()`, `u64()` | `$getrandomU32()`, `$getrandomU64()`, a BigInt |
| `Error::UNSUPPORTED`, `Error::from(code)`, `e.code()` | `2147483648`, `code`, `e` |
| `e.raw_os_error()` | `$getrandomRawOsError(e, "0.2")` |
| `Error::new_custom(n)` (0.3, 0.4) | `131072 + n` |
| `{}`, `{:?}` of an `Error` | `$displayGetrandomError(e, "0.1")`, `$debugGetrandomError(e, "0.1")` |
| `Box<dyn Error + Send + Sync>` of one | `{ value: e, impl: $getrandomErrorDyn("getrandom::error::Error", "0.1") }` |

- **The catalog owns the versions.** `cargo::READ_AS_RUSTC_BUILDS_THEM`
  lists each qualified version, the `--cfg`s rustc's check of it is given,
  and when it isn't given them. 0.1 needs none. 0.2 gets its `custom`
  feature, which its `js` feature's backend comes before. 0.3 and 0.4 get
  `getrandom_backend = "custom"`, unless the check already chooses a
  backend: a `getrandom_backend` cfg the user gives, or 0.4's `wasm_js`
  feature. With both, the custom backend would win and leave the
  `Error::WEB_CRYPTO` that `Display` names undefined, which is a rustc
  error. Each cfg is the crate's own documented opt-in, never a bypass. The
  JS is rust-js's whatever backend rustc checked.
- **A qualified crate says so beside its metadata.**
  `library::qualified(metadata)`, `<rmeta>.rust-js-qualified`, holds the
  version. The catalog's owner writes it as rustc's check succeeds, with
  the metadata Cargo keys and rebuilds. Recognition reads it
  (`recognition::shipped`) to know which version's API a call is. A
  getrandom with none, a plain Cargo build's such as `serde/build.sh`'s,
  is refused where it's called, since its version isn't known. The fact
  is general: memchr's qualified check writes it too, which nothing reads
  yet. memchr's recognition still accepts a memchr without the fact, as
  serde's build gives it; making it read the fact is the one way for later.
- **The fill** writes a chunk of at most 65,536 bytes at a time, as many
  as `getRandomValues` fills at once, into the array or the `&mut` view
  the slice is. Where there's no `globalThis.crypto` it gives `Err` of
  that version's `UNSUPPORTED` and leaves the slice untouched.
- **Its `Error` is its code.** It's a number in every representation:
  `shape`, copies and `==` (`===`), drops, and constants (a constant's
  value tree is its code). `Display` and `Debug` are that version's own
  `fmt`s, read from its `error.rs`: the internal codes' words from each
  version's table (0.3's and 0.4's are the three that rustc's check for
  rust-js's target keeps), `Unknown Error: {}`, and `Error { internal_code,
  description }` by std's `debug_struct`, pretty and given the
  `Formatter`'s options. Written into a writer, each piece is the
  `write_str` the native build gives. An OS's number, which 0.1's and
  0.2's `From<NonZeroU32>` can make, shows the unix build's `strerror_r`
  words, the platform's from ADR 0462's table (`$osErrorOf`), or `OS
  Error: {}` where `strerror_r` knows no name, as glibc and macOS both
  fail then. That follows Tu's decision for `io::Error` and needs Node.
  0.3's and 0.4's API makes no OS number.
- **As a `dyn Error`**, through the existing dyn machinery:
  `std_error_dictionary` gives `$getrandomErrorDyn`, with the same
  `Display` and `Debug`, `source()` `None` as its impl's, and its
  `TypeId`'s key for downcasts. rand_core 0.5's `From<getrandom::Error>`,
  `Box::new(error)`, and `getrandom(dest)?` in `OsRng` compile as rust-js
  compiles rand_core.
- **Everything else of getrandom is refused where it's called**: 0.4's
  `SysRng`, an `Error` made an `io::Error`, the `implementation` module.
  So is a getrandom of another version, a fork, or one by path or git. Its
  refusal now names the qualified version that its own `^` requirement
  takes (`cargo update -p getrandom --precise 0.2.17` for 0.2.15), where it
  named the catalog's first.
- **tooling/compat/covered.txt keeps getrandom's lines, with a new
  meaning.** A line now says the crate's use of wasm-bindgen checks against
  rust-js's patched wasm-bindgen. getrandom's lines (0.1, 0.3 and, new, 0.4)
  are shipped-crate entries, so a graph with getrandom's `wasm_js` and
  chrono's `wasmbind` keeps the patch chrono needs. compat.test.ts's git
  stand-ins named getrandom are now uuid's and chrono's: a getrandom that
  isn't crates.io's is refused.

## Why

- `test/shipped-crates.test.ts` builds `test/shipped/getrandom`, an app of
  each qualified version and rand_core 0.5.1, through `rust-js build`. It
  runs the app beside the native build of the same source, line for line:
  fills of 0 to 70,000 bytes across the chunk's edge, a part of a slice,
  uninitialized bytes, `u32` and `u64`, each `Error` constant and code
  (internal, unknown, custom and OS) shown plain, pretty, padded, a piece
  at a time into a writer and as a `Box<dyn Error>`, downcasts to each
  version's `Error`, and rand_core's `OsRng` and `From`. The same app with
  0.3's `wasm_js` and a user's cfg, and 0.4's `wasm_js`, builds with the
  patch and runs the same. A refusal case covers an `Error` into an
  `io::Error`, `SysRng`, and a plain Cargo build's getrandom.
- `test/registry-crates.test.ts` refuses a git fork of getrandom 0.2.15,
  naming 0.2.17. The cargo.rs unit tests cover the backend rule and the
  compatible version.
- itertools' tests that #2326 listed run as native, so they come off
  `scripts/crater-decided.txt`, held by `crater --check`.

## Consequences

- getrandom's own Crater tests no longer compile to JS, since its crate
  is rustc's, as memchr's is. The crate corpus row says it's shipped.
- A `TypeId` key differed between crates that see different numbers of
  same-named crates (#2353, found here). That's fixed apart.
- Mutations: the backend rule giving the custom cfg always, or never;
  the fact unwritten or unread; a fill of one chunk only; the `uninit` Ok
  without its slice; 0.1's table read for 0.2; the OS's words never read;
  the dyn dictionary dropped for its type's own; and the hint naming the
  catalog's first version.
