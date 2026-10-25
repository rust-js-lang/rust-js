# 0876. A `HashMap`'s declaration is a `Map`, a `HashSet`'s a `Set`

Status: Accepted. Extends [0196](0196-typescript-declarations.md)'s
declarations to std's maps and sets, whose JS [0059](0059-hashmap.md)
made a `Map` and a `Set`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: a declaration says
what the JS is; nothing runs differently.

## Context

React's ReactTaint keeps `TaintRegistryByteLengths`, upstream's
`Set<number>`, which its port holds as `Rc<RefCell<HashSet<u32>>>`. Its JS
was `new Set()`, but its `.d.ts`

```ts
export const byteLengths: any;
```

as was every `HashMap` and `HashSet` a module exported or took: any type
of another crate the declarations didn't know was `any`, which a caller's
TypeScript doesn't check.

## Decision

**A `HashMap<K, V>` or a `BTreeMap<K, V>` is declared `Map<K, V>`, and a
`HashSet<T>` or a `BTreeSet<T>` `Set<T>`, of what each holds as it's
declared** (`declarations.rs`'s `ts`):

```ts
export const byteLengths: Set<number>;
export const fibers: Set<Fiber>;
export function ages(): Map<string, number>;
export function points(): Set<Point>;
```

- **Each is one**: the runtime's classes of a key by value, `$KeyMap` and
  `$KeySet` (ADR 0121), of a path, a `Cow`, or a B-tree's own `Ord`, each
  extend `Map` or `Set`, so TypeScript's methods are each one's.
- **What a B-tree's iteration is isn't said**: TypeScript's `Map` has no
  order of its own; a `BTreeMap` iterates in its keys' order only through
  Rust (ADR 0059).

## Why

- **TypeScript's own types of the JS there is**: a caller is checked, and
  upstream's `Set<number>` is what the port's `.d.ts` says.

## Alternatives

- **`ReadonlyMap` of a `&HashMap`**: a JS caller can change what Rust
  lends it; a shared reference isn't TypeScript's readonly, and nothing
  else of a `&T` is declared so.

## What holds it

- `test/declarations-collections.test.ts`, "a HashMap is declared a Map
  and a HashSet a Set": a thread-local's `Rc<RefCell<HashSet<u32>>>`, a set
  of objects by themselves, a `BTreeMap` of `Vec`s, a `HashSet` of a
  struct by value, a `BTreeSet` taken, each declared, and TypeScript that
  uses them checked.
- Mutations `map-declared-any` and `set-declared-any`
  (`lower/declarations.ts`).
