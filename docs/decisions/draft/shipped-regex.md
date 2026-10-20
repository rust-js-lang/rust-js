# A shipped regex validates Rust patterns before matching

Status: Draft; blocked on the pattern and resource-bound gates below.
A candidate under [the shipped-crate contract](shipped-crates.md).

Case: C ([0262](../0262-when-rust-and-js-disagree.md)) for matching,
validation, offsets and observable errors; D where faithful execution
cannot be established. A different engine is A only after those
obligations are met.

## Context

Rust regex and JS `RegExp` accept different languages and assign different
meaning to some common syntax. For example, Rust's Unicode `\d` includes
Arabic-Indic digits, while JavaScript's `\d`, even with `u`, does not.
Rust match offsets count UTF-8 bytes; JS indices count UTF-16 code units.
Finding `x` in `"éx"` therefore starts at byte 2, not JS index 1.

Rust regex also specifies a worst-case bound for a search. Replacing its
engine with arbitrary native backtracking does not establish that bound.
These obligations appear in the [crate's documentation](https://docs.rs/regex/);
qualification must use the exact version selected by the shipped catalog.

## Decision

**One pattern frontend validates the qualified Rust regex language and
produces a matching plan. It may use `RegExp` only for plans whose
semantics and resource behavior have been established.**

Validation starts with Rust's language, not with whether `new RegExp`
accepts the string. JS-only lookaround or backreferences must not become
valid Rust regexes. Named-group rewriting alone is insufficient; parse
escapes, classes, flags and nested groups before translating.

Each supported operation has an explicit contract:

| Area | Required agreement |
|---|---|
| Pattern syntax | Rust-valid versus invalid syntax, flags, size limits and error behavior |
| Unicode | Character classes, boundaries, case folding, scalar values and qualified Unicode data |
| Matches | Selection order, empty matches, iteration progress and end anchors |
| Captures | Named and numbered groups, absent versus empty captures, correct indexing failures |
| Offsets | UTF-8 byte ranges, including non-ASCII prefixes and supplementary characters |
| Replacement | Rust replacement syntax, group expansion and callback evaluation order |
| State | Repeated calls cannot accidentally share JS `lastIndex` state |
| Resources | The qualified Rust API's documented search bounds and limits |

Do not pass a Rust pattern directly to `RegExp`, expose a raw JS captures
array as the complete Rust API, or use JS replacement syntax unchanged.
Existing owners supply text byte-offset conversions, `Option`/`Result`
shapes, iterator behavior and Rust panics.

An invalid Rust pattern returns the qualified crate's error. A valid but
unsupported pattern produces an explicit rust-js refusal, statically
where known and at construction where dynamic, before matching begins.
It must not become an ordinary `Err(regex::Error)` that makes valid Rust
take an error branch. Runtime refusals remain distinct from Rust panics.

## Open gates

Before acceptance, specify the supported grammar and operations, prove
which matching plans may delegate to the platform, and define the
resource bound for those plans. If broad delegation cannot keep Rust's
bound, implement one faithful matcher or leave the unsupported plans
refused. Do not silently introduce a second public API or a relaxed mode.

Qualify behavior across the supported JS engines; engine-specific
optimizations are not a portable guarantee. The document must name exact
supported features and Unicode/version assumptions before advertising
crate support.

`regex::bytes`, `RegexSet`, builder configurations and `regex-automata`
are not implicitly covered by the first `Regex` implementation. Add each
to the same explicit matrix only after its own contract is tested.

## Acceptance

Run upstream-compatible tests and generated pattern/haystack pairs
against native Rust. Include Unicode digits, word boundaries, combining
marks, supplementary characters, case folding, empty/optional captures,
empty matches, newlines, replacement syntax and repeated searches.
Test literal and dynamically assembled patterns through the same path.

Use adversarial valid patterns and bounded stress inputs to check
resource behavior. Measurements complement the matching algorithm's
bound; passing a few timeouts is not proof of it. Mutations accept
JS-only syntax, use UTF-16 offsets, erase absent captures, preserve stale
`lastIndex`, or delegate an unqualified plan.

Doc examples are smoke tests. The API matrix, refusal tests and generated
cases establish the supported scope; a passing example does not establish
complete regex or regex-automata compatibility.
