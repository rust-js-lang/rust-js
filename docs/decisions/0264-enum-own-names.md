# 0264. A variant's own name is the variant

Status: Accepted. Extends [0013](0013-fieldless-enums.md) and
[0233](0233-match-reads-a-table.md). Amended below: one of the crate's alone,
only called, is written nowhere.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Page names an image after its section, a string:

```tsx
`/images/og-${section}.png`
```

In Rust a section is an enum, and its name a method, as Rust writes one:

```rust
impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Blog => "blog",
        }
    }
}
```

A fieldless variant is already its name in JS (ADR 0013), but rust-js
made the method a conditional and called it, `Section.as_str(section)`.

## Decision

- **A `match` giving each variant its own name is what's matched**: the
  method is `return section;`, as a value or returned, as ADR 0233's
  table read now is returned too.
- **A call of the crate's function whose body is such a `match` of its
  one parameter is its argument**: `section.as_str()` is `section`. An
  option mapped by it, `section.map(Section::as_str)`, is the option.

```js
return `/images/og-${section}.png`;
```

A variant given another name, `Section::Blog => "news"`, is a conditional
and a call, as before.

## Why

- **No program can tell**: each arm gives the string the variant already
  is, and a call of the function gives back what it's given, with no
  effect of its own.
- **It's the JS a person writes**, from Rust's own idiom, which runs
  natively too.
- **It's tested**: a compiler test names a section both ways, its own name
  and another; mutations keep the conditional, the returns, the call, and
  take another name for its own.

## Since

- **So is `o.map(f)` of a binding that's the value itself**, `text.map(js::unknown)`
  is `text`.

## Amendment: one of the crate's alone, only called, is written nowhere (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing reads it.

React's port of ReactInternalTypes gives its `HookType` a `pub(crate) fn
name(self) -> &'static str`, matching each variant to its name (h08b of
the port's gaps). Each call of it is the hook type itself, as this
decision makes it, `${hookName}`, so nothing in the JS reads it; but it
was still written, `export const HookType = { name(hookType) { return
hookType; } }`, in a file upstream has types alone in. Upstream has no
such function.

**A function that gives each variant its own name, which no other crate
reaches (`library::reachable`), is written only where something reads it
as a value** (`pipeline.rs`, with what's written only where it's reached,
ADR 0307): its calls, each what it's given, don't. One a `fn` pointer
holds is written, which that pointer calls; a `pub` one of a library,
which another crate's JS calls, is written as before. So the port's
ReactInternalTypes is upstream's, with no `HookType` object.
`patterns::gives_own_name` is the one test of the function, the calls'
and this one's.

Held by `test/lowering-structs-enums.test.ts`, "an enum's own names of
the crate alone, called, are written nowhere", run; mutations
`own-name-fn-written` and `own-name-fn-of-a-library-unwritten`
(`lower/pipeline.ts`).

## Amendment (2026-10-29): a transparent struct's `From` gives what it's given too

`gives_own_name` is `gives_its_argument`, of either function each call of
which is what it's given: this decision's, and a `From` between a
`#[repr(transparent)]` struct and its field
([0584](0584-a-transparent-struct-is-its-field.md), amended). It's asked
of the impl a trait's method resolves to, and of a library's as its
contract lists.

