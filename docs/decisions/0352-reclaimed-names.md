# 0352. A name given apart is the name where nothing it'd shadow is read

Status: Accepted. Amends [0038](0038-js-names-and-destructuring.md)'s
"the module's own names are never reused".

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A local never took a module's name (ADR 0038), so a destructured prop of
a module function's name was `{ label: label$1 }`, where the function never
read the module's `label`; and a name JS has, `Error`, was named apart
for good, `import Error$ from "next/error"`, where its module never read
JS's `Error`. react.dev writes `{ label }`, and Next.js's docs `import
Error from "next/error"`. The names are chosen as a body is lowered, before
what it reads is known.

## Decision

**Once a module is lowered, a name given apart from another, `x$1` or
`x$`, is that name where its scope never mentions it:**

- **A local's**, where its item, a function, a type's methods, or a
  constant's value, neither declares nor reads the name anywhere.
- **An import's**, where its module never does.
- **Never a word JS keeps**, `class$`; a global rust-js names apart,
  `Error`, `String`, `Math`, may be the name, where nothing reads the
  global, a template's conversion to a string, `String(n)`, among the
  reads.
- **Not an `on_load!` body's**, whose names are the module's (ADR 0267).

```js
export function show({ label }) { .. }      // was { label: label$1 }
import Error, { catchError } from "next/error";
```

## Why

- **It's the same program**: a name renamed to one its scope never
  mentions keeps every read of it its own, so nothing can tell, but the
  reader.
- **It's what JS writes**: a parameter shadows a function it doesn't call,
  and an import takes a global's name it doesn't use.

## Consequences

- Seven corpus cases, five snapshots, a JSX snapshot, and three react.dev
  modules read shorter: `match`, `entry`, `{ value }`, `<OpenInCodeSandboxButton />`
  of an import named apart from nothing it reads; 0 of react.dev's 823
  pages differ.
- A name its scope reads elsewhere keeps its `$1`, `f$1` beside an `f`.
- An import named apart from another module's import of its name, which
  naming crate-wide avoids (ADR 0202), is reclaimed too: the mutation that
  named it apart is no longer caught, and is gone.

## Amendment: a local's name is checked by its reads

A local is renamed where every read in its item is of the declaration it
was, as JS's scopes resolve them (ADR 0357), not only where its item never
mentions the name. An import keeps the rule above.

## Amendment: a module's own function or `const`

A function or `const` of a module's own that it doesn't export is
reclaimed as an import is, where its module never mentions the name:
react.dev's MDXComponents writes `function Math({children})`, which was
`Math$`. One another module imports keeps its name, which the importer
reads it by, and one in a module that reads the global, `Math.imul`,
stays apart. A lowering test has all three; mutations keep the name
apart, and reclaim an exported one.

## Amendment: every JS global the output may read

The names named apart were `Math`, `Error`, `String` and a few more, so a
Rust name `console`, `Map`, `Set`, `Array` or `TextEncoder` kept its name
and hid the global the output reads: `const console = 3; console.log(..)`,
`const Array = new Array(3)` (#142). `GLOBALS` is every ECMAScript global
and the hosts' rust-js reads, `console`, `TextEncoder`, `globalThis` and
the rest; reclaiming gives the name back where the module never reads the
global, so a program that doesn't sees no change. Held by
`test/corpus/js_global_names.rs`.

## Amendment: an arrow the compiler makes around the program's values

An arrow lowering makes around a value the program gives, `next_if_eq`'s
test `(item) => item === x`, `inspect`'s `(item) => { f(item); .. }`, a
sort's by a key, `contains`, `binary_search`, a `for_each` fold's, a char
set's pattern, `String::retain`'s and an `Option`'s or `Result`'s setter,
named its parameter by a word of its own: where that value read the same
name, a variable the program calls `item`, the parameter hid it, and
`next_if_eq(&item)` was always true (#1667, a wrong answer). Its name is
`Expr::param_apart(base, embedded)`'s: the word, or given apart from each
name the values it holds read, `(item$1) => item$1 === item`; nothing
changes where none does. Held by
`test/corpus/arrow_parameters_named_apart_from_program_values.rs`, native
compared, and mutation `param-shadows-what-its-arrow-holds`. That a name
is the identity of what it reads, which this rule works within, goes with
review #1671's change 3 (per-item variable ids).

Amended 2026-11-14 (#1681 change 3, M2): such a parameter is the item's
own name, `FnCx::fresh`'s, and the item's names hold each name a value it
embeds can read: the item's variables, its module's functions and
imports, and the globals it binds. `Expr::param_apart`, which gave it
apart from the names the value read, is gone with the check, which no
value could reach any more; mutation `param-shadows-what-its-arrow-holds`
is gone with it. A global the value reads, `next_if_eq(&ITEM)` of a
`#[link_name = "item"]` static, holds the rule in `test/validate.test.ts`.
