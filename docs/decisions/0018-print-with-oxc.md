# 0018. Print with oxc, through one adapter file, and emit source maps

Status: Accepted. Supersedes the printer half of [0007](0007-js-ast-and-printer.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Our own printer (`js.rs`, ~320 lines) handled precedence and indentation
well, but three needs were coming:

- **Source maps**, so a debugger can step through the Rust, not the JS.
- **Correct string escaping**, which string literals will need. `{:?}` isn't
  JS escaping.
- Eventually **minified output**, and maybe `.d.ts` typings.

[oxc](https://oxc.rs) is a JavaScript toolchain written in Rust. Its code
generator, `oxc_codegen`, already handles precedence, escaping, number
formatting, minification and source maps.

## Decision

Keep our small JS AST, and convert it to oxc's AST in **one file**, which
oxc then prints:

```
THIR ──lower.rs──► js.rs AST ──to_oxc.rs──► oxc AST ──oxc_codegen──► .js + .js.map
                   (ours, with spans)       (the only file that uses oxc)
```

- **Every node in our AST carries a span**: byte offsets into the `.rs` file.
  `Span::NONE` (empty) means "no mapping", which oxc skips.
- **The source map points into Rust** through one trick: oxc computes each
  mapping's line and column from `program.source_text` plus the node's span.
  We pass the Rust file as `source_text`, and spans that index into it. oxc
  doesn't care that the "source" isn't JavaScript.
- **Pinned versions**: `oxc_* = "=0.151.0"`, `oxc_sourcemap = "=8.1.2"`.
- rust-js writes `out.js` and `out.js.map`. The JS ends with
  `//# sourceMappingURL=out.js.map`. The map names the Rust file relative to
  itself (`../examples/fib.rs`) and embeds its text (`sourcesContent`).

## Why

- **Our AST stays small and pleasant.** `lower.rs` keeps writing
  `Expr::bin(Op::Add, l, r)`. oxc's builder needs an arena, a lifetime, and
  a dozen arguments per function node. That verbosity lives only in `to_oxc.rs`.
- **oxc churn is contained.** oxc is pre-1.0. Between versions it renamed its
  whole builder API (`ast.expression_binary(..)` became
  `Expression::new_binary_expression(..)`) and split export nodes apart. When
  it changes again, we fix one ~300-line file.
- **Source maps nearly for free.** oxc already tracks output positions. We
  only had to supply spans.

## Details worth knowing

- **Rust spans → file offsets** (`FnCx::js_span`): rustc numbers bytes
  across all loaded files, so we subtract the root file's start. Code from
  a macro or desugaring maps to where it was written (`source_callsite`), so
  a `while` loop maps to the `while` keyword. Spans outside the root file get
  `NONE`.
- **rustc's copy of the source**, not a fresh read from disk: rustc
  normalizes the text it parses (it strips a BOM and turns CRLF into LF), and
  our byte offsets are into *that* text.
- **The prelude** (header comment and runtime helpers) is plain text above
  oxc's output. It has no mappings.
- **Blank lines between functions**: oxc prints top-level functions back to
  back. `to_oxc.rs` inserts a blank line before each function, then rebuilds
  the map with every mapping moved down by the number of lines inserted
  above it (`shift_lines`). The prelude shift uses the same mechanism.
- **Whole numbers print as written.** oxc prints numbers in their shortest
  form, like a minifier (`1000` becomes `1e3`), with no option to turn that
  off. `to_oxc.rs` hands oxc whole numbers as ready-made decimal text instead,
  wrapped in a real unary minus when negative, so oxc still handles spacing and
  parentheses. Fractions keep oxc's formatting.
- **Where a mapping points**: the outermost JS node made for a Rust
  expression gets that expression's span (`Expr::or_at`). Helper nodes like a
  `| 0` wrapper share it. Statements get their Rust statement's span.
  `return a;` from `break a;` maps to the `a`, on the right line.

## Alternatives

- **Keep our printer, add source maps ourselves**: we'd reimplement
  position tracking, VLQ encoding and string escaping, all of which oxc
  already does well.
- **Build oxc's AST directly in `lower.rs`**: one fewer layer, but it spreads
  the verbose API and oxc's version churn through the whole compiler.
- **swc**: a mature alternative with the same capabilities. We went with
  oxc. Because of the adapter, switching later would touch only
  `to_oxc.rs`.

## Consequences

- **Tests**: `bun test` now checks the map, not just the behavior: the
  sources entry, embedded content, no mappings in the prelude, every
  mapping inside the Rust file, and eight JS-snippet → Rust-snippet probes.
  A deliberate mutation (dropping the line shift) makes it fail, so it
  guards real behavior.
- **Build cost**: oxc adds dependencies. A clean build is slower. Incremental
  builds are unaffected.
- The map names nothing (amendment below).
- **Known gap**: declared names (`const x = ..`, parameters) have no span of
  their own. The whole statement is mapped instead, which is enough for
  line-level stepping.
- Minified output is now one option away (`CodegenOptions::minify`), but isn't
  exposed yet.

## Amendment: the map names nothing

Case: N ([0262](0262-when-rust-and-js-disagree.md)): what the program does
is the same; this is what tools show of it.

**A source map rust-js writes has no `names`.** oxc names each identifier
it prints by the source text its span covers, where that differs from the
identifier. rust-js's spans are the Rust a JS node came from, rarely an
identifier alone: a parameter's is its declaration, `callback: impl
FnOnce(A, B) -> T`; a closure that became the function it calls,
`runWith(beginWork, ..)`, has the closure's text; `beginWork`'s is
`begin_work`. A stack trace through the map shows such a name as the
frame's: source-map-support, Jest's, names a frame by the name at the
position it's called from, so a frame through React's port showed eleven
lines of Rust, `Some(Rc::clone(unit_of_work)), |current: ..| { .. }, ..)`,
and React's own tests of an error's stack failed (the port's gap wl5).

A name is what a tool shows for the JS identifier, so it would be right
only as the JS's own name, which oxc leaves out, or as nothing. Without
one, a frame is named as V8 names the JS, `beginWork`, as a person's JS
would be, and the map still takes each position to its Rust.
`restore_sources` in `src/to_oxc/expressions.rs`, where oxc's map becomes
rust-js's, carries no name; formatting's remaps copy what it carries.

Alternatives: names kept where the span is a Rust identifier alone,
`begin_work`, which a debugger could show in place of `beginWork`; but a
stack trace would show it too, a name the JS doesn't have, and spans that
are an identifier alone aren't always that identifier's (a temporary held
for a value). Names kept where they're valid JS identifiers: the same, and
a parameter `a` whose span is `a: A` would still lose its name.

`test/emission.test.ts`'s "a source map names nothing" compiles a renamed
function, a closure that becomes the function it calls and parameters with
types, and reads the map; mutation `source-map-names-kept`.
