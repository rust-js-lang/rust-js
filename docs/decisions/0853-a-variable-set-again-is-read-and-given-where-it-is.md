# 0853. A variable set again is read, and given what's made, where it is

Status: Accepted. Narrows the handle held for a `Cell` reached through a
reference written again (MIR's lowering, ADR 0364); extends
[0650](0650-a-shadow-of-a-binding-read-no-more-is-that-binding.md)'s
shadows to a binding the variable it shadows is given last; and extends
prepare's `const n$1 = n;` of a closure's parameter to what a closure
declares.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for each. What's read
and what's set are the same values at the same points; only temporaries
and names given apart go.

## Context

React's `cache` walks its nodes, `cache_node` set again as it goes:

```rust
let mut cache_node = fn_node;
for &arg in arguments {
    if let Kind::Function(_) | Kind::Object(_) | Kind::Array(_) = js::classify(arg) {
        let arg: &JsObject = unsafe { js::cast(arg) };
        let object_cache = match cache_node.o.get() {
            None => {
                let object_cache = weak_map::new();
                cache_node.o.set(Some(object_cache));
                object_cache
            }
            Some(object_cache) => object_cache,
        };
        let object_node = object_cache.get(arg);
        cache_node = match object_node {
            None => {
                let cache_node = create_cache_node();
                object_cache.set(arg, cache_node);
                cache_node
            }
            Some(object_node) => object_node,
        };
    }
}
if cache_node.s.get() == Status::Terminated {
    return cache_node.v.get();
}
```

Its JS held what `cacheNode` was before each read, made a binding of each
name twice, and copied the closure's `arg`:

```js
const arg$1 = arg;
const cacheNode$1 = cacheNode;
const objectCache$1 = cacheNode$1.o;
let objectCache;
if (!objectCache$1) {
  const objectCache$1 = new WeakMap();
  const cacheNode$1 = cacheNode;
  cacheNode$1.o = objectCache$1;
  objectCache = objectCache$1;
} else {
  objectCache = objectCache$1;
}
const objectNode = objectCache.get(arg$1);
if (!objectNode) {
  const cacheNode$1 = createCacheNode();
  objectCache.set(arg$1, cacheNode$1);
  cacheNode = cacheNode$1;
} else {
  cacheNode = objectNode;
}
// ..
const cacheNode$1 = cacheNode;
if (cacheNode$1.s === Terminated) {
  const cacheNode$1 = cacheNode;
  return cacheNode$1.v;
}
```

Upstream's reads `cacheNode` where it reads it, and sets it where it
makes the node.

## Decision

```js
let objectCache = cacheNode.o;
if (!objectCache) {
  objectCache = new WeakMap();
  cacheNode.o = objectCache;
}
const objectNode = objectCache.get(arg);
if (!objectNode) {
  cacheNode = createCacheNode();
  objectCache.set(arg, cacheNode);
} else {
  cacheNode = objectNode;
}
// ..
if (cacheNode.s === Terminated) {
  return cacheNode.v;
}
```

1. **A `Cell` reached through a reference written again is read through
   it where it's read, by its `get`, `set`, `replace` or `take`, before
   anything writes the reference** (`mir/queries.rs::read_before_written`,
   asked by `mir/rvalues.rs`). MIR makes `&(*cache_node).s` a temporary,
   which the call reads; where every way from it reaches that call before
   an assignment of `cache_node`, `cache_node.s` read there is the `Cell`
   it was. A handle a variable keeps, `let v = &node.v;`, one given to what
   may keep it, `hold(&node.v)`, and one read after `node = b`, `node.v.set({
   node = b; .. })`, stay held, `const node$1 = node;`, as before. Borrowck
   lets no `&mut` to the reference write it between: it's read first.
2. **A binding of a variable's own name, given apart, that the variable
   is given after it in its block is the variable, given it where it's
   made** (tidy's `given last`): `const node$1 = make(); m.set(k, node$1);
   node = node$1;` is `node = make(); m.set(k, node);`, as `node = match ..
   { None => { let node = make(); ..; node } .. }` reads. Only of a
   variable the body declares, or its parameter: a closure's capture,
   `x = { let x = k * 10; check(k); x }` in `catch_unwind(|| ..)`, is read
   as it was by its caller once `check` unwinds. Only where nothing
   between names the variable, no `break` or `continue` between may leave
   before it's given, no closure or handle reads either, which may run
   between, and not in a `try`, whose `catch` or `finally` may read the
   variable as it was (none known can: a drop that reads it is of its own
   old value, whose drop makes the assignment another form; kept as a
   bound, untested).
3. **A variable given what's read before, one way, its own value given
   first the other, is given what's read** (tidy's `filled lazily`): `const
   objectCache$1 = node.o; let objectCache; if (!objectCache$1) { ..fill.. }
   else { objectCache = objectCache$1; }` is `let objectCache = node.o; if
   (!objectCache) { ..fill.. }`. Only of what's read named as the variable
   is, given apart: a binding the program names, `const fnNode =
   fnMap.get(fn);`, stays. The way that fills it reads it only once it's
   given its value, as Rust reads no variable before.
4. **A closure's variable shadowed by its own value is that variable**:
   prepare's `const n$1 = n;`, which a closure's parameter's was
   already, of what the closure declares too, `for (const arg of
   arguments$)`'s `arg` of `let arg: &JsObject = unsafe { js::cast(arg) }`.
   Nothing outside a closure sets what it declares.

## Alternatives

- **`cacheNode.o = objectCache = new WeakMap();`, upstream's chained
  assignment**: one statement, but a form a person writes rarely and the
  Rust doesn't; deferred.
- **Fold a held temporary in tidy, `const node$1 = node;` read before
  `node` is set**: tidy can't tell a temporary MIR held for a handle from
  a binding the program names, `let terminated_node = &cache_node;`, which
  upstream names too. MIR can.

## Consequences

- `test/snapshots/playground`'s `directoryOf` and `App` give `name` and
  `file` what's split where it's split, no `name$1`.
- The readability baseline (`docs/readability.txt`) is blessed: `while
  (true)` grew from 16.1 to 16.2 in 10,000 lines, the loop of
  `given_last_unwinds`' `replaced`, whose drops are a `try` (ADR 0437).
- Found writing this, not fixed here: a `Cell` read through a reference
  set through a raw pointer to it, `let p = &raw mut node; node.v.set({
  unsafe { *p = b }; 1 })`, sets `b`'s; the variable is a cell (ADR 0407)
  whose writes `written_again` doesn't count, before and after this.

## What holds it

- `test/corpus/cache_node_walk.rs`: the walk, a handle kept past a
  reassignment and past a closure's, a `continue` before what's made is
  given, a closure reading the variable before, and a cache filled by `if
  let` and by a `match` of a binding the program names, with native Rust.
- `test/corpus/given_last_unwinds.rs`: a panic between what's made and
  the variable given it, dropping both; a closure's capture given what it
  makes, read after it unwinds; and a `Cell` read after its reference is
  set again, or given to what keeps it, a function or a method of its own,
  with native Rust.
- `test/lowering-shadows.test.ts`, "a closure's loop variable shadowed by
  its own value is that variable": React's `cache`'s cast `arg`.
- Mutations in `scripts/mutations/lower/mir/rvalues.ts`
  (`cell-read-before-written-held`, `cell-kept-by-any-call`),
  `scripts/mutations/lower/mir/queries.ts`
  (`read-before-written-past-a-write`),
  `scripts/mutations/lower/mir/tidy/bindings.ts` (`given-last-past-a-jump`,
  `given-last-of-a-captured`, `given-last-of-a-capture`,
  `given-last-of-another-name`, `filled-lazily-not-negated`),
  `scripts/mutations/lower/mir/tidy/stages.ts` (`given-last-unasked`,
  `filled-lazily-unasked`) and `scripts/mutations/prepare.ts`
  (`closure-declared-shadow-kept`).

## Amendment: a pattern's names aren't asked

A closure's loop variable shadowed by its cast, `let arg: &JsObject =
unsafe { js::cast(arg) };`, was `const arg$1 = arg;`, which preparation
made `arg` again as the closure's own name, a `for…of`'s pattern's. MIR
gives such a binding the name it binds already: no `const x$1 = x;` of a
`for…of`'s or a destructuring's name is made, of a cast, a deref
coercion or a clone, in a closure or not, and no corpus case's shadow was
decided by one, so `closure-declared-shadow-kept` survived every run.
Shadows ask a pattern's names no more; the test of the cast keeps a call
between it and its read, so a shadow made again would show as `arg$1`.
