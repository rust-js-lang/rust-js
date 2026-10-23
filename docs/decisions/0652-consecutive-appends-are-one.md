# 0652. Consecutive appends to a `String` are one `+=`

Status: Accepted. Extends [0034](0034-strings-and-chars.md), whose
`push_str` and `push` are `s = s + t`, which preparation writes `s += t`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: no program can tell.

## Context

React's `DefaultPrepareStackTraceV8` adds each call site to the stack in
one statement:

```js
for (let i = 0; i < structuredStackTrace.length; i++) {
  stack += '\n    at ' + structuredStackTrace[i].toString();
}
```

Its port appends the two as Rust does, each a `push_str`:

```rust
for call_site in structured_stack_trace {
    stack += "\n    at ";
    stack += &call_site.to_string();
}
```

and rust-js wrote each, `stack += '\n    at '; stack += callSite.toString();`.

## Decision

**A `String` of the body's own appended to twice, one statement after the
other, `s = s + a; s = s + b;`, one of them text written out, is `s = s +
(a + b)`, `s += a + b`**; of two pieces of text written out, one, `t +=
"!?"`. Of one value and the text around it, `out += name + ";"`, as JS
writes a line built up; two values stay apart, `out += a + "\n"; out += b
+ "\n";`, not one statement of every line. Lowering's `text::appended`
writes it, the `String`'s own rewrite that tidy calls once the body is
named (`tidy::named`), as a map's counted entry is the map's. Its locals'
types say which are `String`s (`Locals::text`).

- **Of text alone**: `a` and `b` are `&str`s or `char`s, JS strings, so
  `a + b` is their text, and `s + (a + b)` is `(s + a) + b`. A number's `f
  += 1; f += 1;` isn't: `f + 2` rounds once where `f + 1 + 1` rounded twice;
  text written out is never a number's, nor is a `String`.
- **In the same order**: JS's `+` makes `a`, then `b`, as the two did; `b`
  is made before `a` is appended now, so **not where `b` reads `s`**, `s +=
  s`, which would see `s` without `a`. Nothing else can see `s` there: Rust
  lends it to nothing while it's changed, so no closure, handle or call
  reads it between.
- **Not in a `try`**, whose `catch` or `finally` could see `s` without `a`
  where `b` throws: a fallible `write!`'s pieces, appended one by one for
  the `catch` to add what's written (ADR 0187), say. No Rust program's
  appends reach one today, as a `write!`'s are in a closure and a JS
  call's `try` holds only what can't throw; the rule says so anyway, where
  the next one might.
- **Of a `String` of the body's own**: a closure's of a `String` it
  captures is its function's, which may read it after a throw, where a JS
  call's `try` (ADR 0035) catches the panic of a closure it called.

The last two hold where no program rust-js compiles today reaches them:
no test can make either fail, so no mutation holds them.

```js
for (const callSite of structuredStackTrace) {
  stack += "\n    at " + callSite.toString();
}
```

## Why

- **No program can tell**: the string is the same text, its pieces made
  in the same order, and nothing reads it between them.
- **It's the JS React writes.**
- **One owner**: what an append to a `String` is, `text.rs`'s, which has
  `push_str`'s lowering; tidy only says when.

## Consequences

- Two corpus cases' text written out is one, `s += "abc-7"`; three
  snapshots' and two guide pages' a line's value and its end, `s +=
  String(i) + ",";`, `out += name + ";"`.

## What holds it

- `test/lowering-strings.test.ts`, "consecutive appends to a string are
  one +=": React's `prepareStackTrace`, run in Node as V8's hook; text
  written out; an append that reads the string, a number's, and two
  strings' appends, each run.
- `test/node-bindings.test.ts`'s V8 hook.
- Mutations in `scripts/mutations/lower/std_types/text.ts`: the appends
  kept, one that reads the string joined, two strings', and text joined
  the wrong way round.
