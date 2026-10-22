# 0592. An impl's dictionary is its module's `const`

Status: Accepted. Amends [0049](0049-traits-and-generics.md), whose
dictionary was a function caching it in a `var`, and
[0019](0019-one-js-file-per-module.md)'s "cycles are fine". Decided with
the user.

Case ([0262](0262-when-rust-and-js-disagree.md)): A where it compiles: a
dictionary made where its module loads is the one every use reads. Where
a module that runs code when it loads shares an import cycle with one
whose dictionary that code may reach, the program is refused (below).

## Context

ADR 0049 wrote each impl's dictionary as a function that made it the
first time it was asked for, in a `var` declared at the module's top:

```js
var $circleShape;

export function circleShape() {
  if ($circleShape === undefined) {
    $circleShape = { area: circleShape_area };
  }
  return $circleShape;
}
..
total(shapes, circleShape());
```

The `var` let a module in an import cycle that loaded first call it before
its own module ran. No one writes `var` today, nor a getter of an object
that never changes: a person writes the object.

## Decision

**An impl's dictionary that takes nothing, no bound's dictionary, const
parameter, type fact nor drop (`takes_evidence`), is its module's
`const`**, read as it is:

```js
export const circleShape = { area: circleShape_area };
..
total(shapes, circleShape);
```

What its entries share, a writer's `write_char` its `write_str`, is the
module's `const` beside it, named apart by the dictionary's reserved name,
`$smallWrite_writeStr`. **One that takes something is a function of it,
its cache its module's `const`**, `const $wrapperDisplay = new
WeakMap();`. No `var` is written; the module's list of caches is gone.

**A module's own load-time code reads a dictionary after it's made**:
items are written in the order what's made when the module loads reads
them (`in_load_order`).

**A dictionary is refused where its module and another that runs code
when it loads, a `js::on_load!` or a value made by a call, import each
other** (a dictionary, its cache and what its entries share run nothing
of the program's, so a cycle of them alone, either's `lib` and
`iterator`, compiles): that module may load first and reach the dictionary before it's
made, where JS would throw a `ReferenceError` that Rust has no cause for.
The error names both modules: move the impl or the load-time code out of
the cycle.

## Why

- **It's the JS a person writes**: an object, made once, read by name.
- **One answer**: whether a dictionary takes anything is `takes_evidence`'s,
  asked where it's made and where it's read.

## Alternatives

- **Keep the `var` and the getter**: cycle-safe, but no one writes it.
- **Order the cycle's modules**: JS loads an import cycle in the order its
  imports are reached, which no module can choose.

## Consequences

- 198 corpus cases' dictionaries are their objects; the corpus's JS is
  2,475 lines shorter, so each shape's share in 10,000 lines grows where
  its count doesn't: `while (true)` 16.9 to 17.4, `$iter` 62.6 to 64.4,
  `return;` 9.5 to 9.7, labeled blocks 2.4 to 2.5, renames 838.7 to 862.8
  of 7,358 where they were 7,360.
- A program whose dictionary sits in an import cycle with a module's
  load-time code is refused where it compiled.

## What holds it

- `test/lowering.test.ts`, "a dictionary that takes nothing is its
  module's const": a dictionary, its module's thread-local made of it,
  a generic one's cache, one of a type's fact, and the cycle refused.
- Mutations: a dictionary a function again, one read called, each kind of
  evidence unseen, the cycle allowed, a load-time call unseen, the cache
  never made, a cache taken for load-time code.
