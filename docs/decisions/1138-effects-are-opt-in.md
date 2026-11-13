# 1138. Effects are opt-in: JS's own calls are marked where lowering makes them, and what a value's text runs is called

Status: Accepted (2026-11-13, #1674; architecture review #1671's change
3, step 1, the user's decision). Supersedes the name lists of
[0890](0890-a-bindings-method-is-known-as-one.md) and
[0900](0900-a-dictionarys-method-is-the-programs.md); beside
[0892](0892-what-only-reads-by-rusts-types-does-nothing.md), what only
reads by Rust's types, and [0312](0312-shown.md), what a template shows.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change where a
call or a value's text was taken to run nothing and ran the program's
code, #1663 and #1664, so Rust's behavior is preserved; elsewhere the
output is the same, but where a class's text is now its call.

## Context

What a JS node may do was decided by names. `Expr::calls_only`, the one
answer `only_reads` and `runs_program_code` give, and so tidy's hoisting,
the format order (`formats_out_of_order`) and the load-cycle refusal
(ADR 0592), took a member call named as one of 23 JS methods, `map`,
`at`, `join`, or a call of a JS global's name, `String`, `Math`, for JS's
own, which runs nothing of the program's. ADRs 0890 and 0900 each closed
one caller class with a flag of its own, a binding's method
(`Known::Binding`) and a dictionary's (`Known::Evidence`). The crate's
own method so named was neither: `Grid.map(g)`, which changes `g`, let
tidy hoist `(g.count, 1)` ahead of it, `11 (1, 1)` where native Rust
prints `11 (11, 1)` (#1663). And a `#[rust_js::class]`'s `Display::fmt`,
its prototype's `toString`, was interpolated, `${p}`, and taken for a
read: `println!("{} {}", p, loud(2))` printed `fmt 1` before `arg 2`
(#1664). A minifier, Oxc's in Vite's production build as esbuild, takes a
value's `toString` to run nothing too, so it moved a held argument past
`${p}` and would drop a format whose text is discarded. A name the
program's function may share says nothing of it, nor does a template say
what its value's text runs.

## Decision

What runs nothing is what lowering knew it to be, said where it made it;
anything else may do anything.

- `Known::BuiltIn` (Permits): a call of one of JS's own functions or
  methods, built where lowering knows what it calls,
  `Expr::builtin(of, "join", args)`, `Expr::call(String, ..).built_in()`;
  or a JS global it names, `Expr::js_global("Math")`, each of whose
  methods is JS's own, `Math.max(a, b)`, whatever lowering calls it by.
  It runs nothing of the program's but the callbacks it's given, and may
  throw. `Expr::is_js_own_call` is the one predicate: `BuiltIn` on the
  call or on the global it calls, or `Read`'s `str` or slice method (ADR
  0892). `calls_only`'s method and global name lists are gone; so is tidy's
  `read_past_calls` list of global objects, which asks `BuiltIn` or
  `Known::Lasting` of the variable: `console`, whose `log` prints, is
  `Lasting`, a global no call changes, never `BuiltIn`. `Known::Binding` is
  gone, which no reader asks now: a binding's method is a call like any
  other.
- What a value's text may run is a call, never a template's
  interpolation: a class's `Display::fmt` is called as the crate's writer
  is, `p.toString()`, its padding `p.toString({ width: 4, align: "Right"
  })`, a `T: Display` given one `(value) => value.toString()` (one form for
  every class's text, the user's decision, through m6-react). So a
  template's interpolation runs nothing: what it shows is text, a number,
  a `bool`, or what a call gave, and `calls_only`, `has_effects` and
  `may_throw` see the code a value's text runs as the call it is.
- Where `BuiltIn` is marked: each call of a JS method or global lowering
  builds by name, rewritten to `Expr::builtin` and `built_in()`
  mechanically, so the calls the old lists matched are the ones marked,
  the crate's excepted.

## Consequences

- The corpus, 2,400 cases, is byte-identical but
  `test/corpus/own_method_named_as_a_js_one_runs_where_called.rs`
  (#1663), `test/corpus/class_display_interpolated_runs_in_order.rs`
  (#1664), each held against native Rust in both builds and the
  production build, its discarded formats among them, and
  `class_to_string`, whose class's text is its `toString()` called, whose
  `T: Display` is `(value) => value.toString()`, and whose argument,
  `leftDiagonal()`, is evaluated before the class is formatted, as Rust
  evaluates every argument first. The guide's examples are the same.
- A new builder of a JS call is "may do anything" until it says
  otherwise: held apart, never moved, which is right, if plainer output is
  lost; its mutation shows it.
- Mutations: `any-call-taken-for-js-own`, `built-in-fact-unread`
  (`js/expr_walk.ts`); `js-global-unknown-to-tidy` (`lower/mir/tidy.ts`,
  held by lowering-strings' read of `Math.PI`); `console-unread-past`
  (`lower/format_args.ts`).
