# rust-js design docs

rust-js compiles Rust to readable JavaScript. The whole design follows from one
sentence, borrowed from ReScript:

> Keep the language's own front end. Replace only the back end. Where the
> language and JS disagree, pick the JS behavior that keeps the output small
> and readable, and write that choice down.

To write an app in rust-js, read [the guide](guide/README.md): it says
what's current. This folder is where we write the choices down, and why. How a disagreement is
decided, by what the program can observe: [0262](decisions/0262-when-rust-and-js-disagree.md).

For what Rust does under rust-js today, read
[How Rust behaves in rust-js](guide/semantics.md), and for what it refuses,
[the known limitations](guide/limitations.md). The decisions below record why,
as each was made: a later one may lift an earlier one's "not yet" or "still
an error", and the page says what's current.

## The pipeline in one picture

```
 fib.rs
   │
   ▼
 rustc front end  (parse, expand macros, resolve names, type check)
   │
   ├──► THIR of every function ──copy──┐        (0004)
   ├──► MIR of every body ─────copy────┤        (0364)
   ▼                                   │
 rustc analysis (borrow check, lints)  │
   │                                   │
   ├── any error? ──► stop, write nothing  (0006)
   │                                   │
   ▼                                   ▼
 Compilation::Stop               mir.rs: MIR ──► JS AST, with spans  (0364)
 (no rustc codegen)                    │  THIR read beside it: names, spans, facts
                                       ▼
                                 mir/tidy.rs: written as a person writes it
                                       │
                                       ▼
                                 link.rs, prepare.rs: linked, then readable
                                       │
                                       ▼
                                 to_oxc.rs: JS AST ──► oxc AST           (0018)
                                       │
                                       ▼
                                 format.rs: laid out ──► one .js + .js.map per module  (0018, 0019)
```

## Code map

What each part of the code is, and where, is
[architecture.md's "Where the code is"](architecture.md#where-the-code-is):
the one map, beside the boundaries it keeps and where a change goes.

## Decisions

Each record says what we decided, why, what we rejected, and what it costs:
[the index of decisions](decisions/README.md), by group.

## Research

Explorations that aren't decisions yet:

- [An in-browser rust-js playground](research/in-browser-playground.md): run rustc's front end + rust-js as WebAssembly
- [Type foundations](research/type-foundations.md): how TypeScript, Scala.js, ReScript and rust-js type JS from the language up to React, and what on par takes
