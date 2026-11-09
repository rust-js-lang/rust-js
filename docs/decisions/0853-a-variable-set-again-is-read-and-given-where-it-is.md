# 0853. A variable set again is read, and given what's made, where it is

Status: Accepted; amended (a closure made after the variable is given,
below). Narrows the handle held for a `Cell` reached through a
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

## Amendment: a closure made after it's given

React DOM's `ReactDOMEventHandle` fills a target's set of listeners the
first time it's asked for, then returns what takes one out again:

```rust
let target_listeners = match get_event_handler_listeners(target) {
    Some(listeners) => listeners,
    None => {
        let listeners = js::set::new(None);
        set_event_handler_listeners(target, listeners);
        listeners
    }
};
target_listeners.add(listener);
Box::new(move || target_listeners.delete(listener))
```

The returned closure captures `targetListeners`, so the variable wasn't
given what's made where it's made:

```js
let targetListeners = getEventHandlerListeners(target);
if (targetListeners == null) {
  const targetListeners$1 = new Set();
  setEventHandlerListeners(target, targetListeners$1);
  targetListeners = targetListeners$1;
}
```

**Only a closure or a `&mut`'s handle made before the variable is given
keeps it apart**: one that exists while what's between runs, which may
call it, or may throw to what calls it later. One made after can't be
called before it's made, and isn't made where what's between throws.
`given_last` passes each block the captures of what's made before it,
its own statements' before `x = y;`, and of each statement around it,
whole, so a loop's, made again before its body runs again, and a
`switch`'s earlier cases', which one may fall into the next from, count.
So upstream's `targetListeners = new Set();
setEventHandlerListeners(target, targetListeners);`. Case: none; what
runs between reads the variable through no closure that exists.

- `test/corpus/given_last_closure_after.rs`: the listeners filled
  lazily and a closure made after, given where made; a closure made
  before the `if` around it, called between, reading what the variable
  was, with native Rust. `cache_node_walk`'s closure before, in the
  block.
- Mutations `given-last-made-before-in-its-block-unseen`,
  `given-last-made-around-unseen`; `given-last-of-a-captured` and
  `given-last-of-a-capture` moved.

## Amendment: a box written through is set again (#980)

A box is the value it holds (ADR 0023), so `*b = v` sets `b` again, as
`b = v` sets a variable. The write count already said so (`whole`), but
three rules asked only whether a statement assigned the local itself:

- `once_each_way` took a box written through after its `let` for one
  assigned once, so its value read the same anywhere (`alike`), and a read
  of it was moved past the write: `pair(*n, { *n += 1; *n })` gave `(6, 6)`
  where Rust gives `(5, 6)`.
- `read_before_written` didn't see `*node = b` as the box's write, so a
  `Cell` reached through `Box<&Node>` was set on the new node.
- A place reached through a reference the variable holds, written again,
  is held where it's made (`const node$1 = node`). That rule asked whether
  the local's type is a reference. A `Box<&T>`'s isn't, so it read the new
  reference. `through_local_reference` now looks through boxes too.

Each now asks `whole`, the one rule for what's the local's value. So
`written_apart` no longer asks whether the program named the local: that
check stood in for MIR's `vec!` box, filled through after it's made, which
`once_each_way` now counts as written again itself. The whole corpus's JS
is the same without it (mutation `mir-alike-machine-once`, removed). Results
changed ([0262](0262-when-rust-and-js-disagree.md)), so Rust's order is
kept. Held by the corpus case `box_read_before_its_write`, the mutations
`once-each-way-box-content-unwritten`,
`read-before-written-box-content-unwritten` and
`local-reference-not-through-a-box`, and the metamorphic net's boxed
rewrites.

## Amendment: a box held in a field, or a field of a box, is its local's (#1033)

The metamorphic net's `field` rewrite (#1008) moved #980's box into a
struct's field, `let mut n = MetamorphField(Box::new(5))`, and the read
was moved past the write again: `pair(*n.0, { *n.0 += 1; *n.0 })` gave
`(6, 6)`, and a field of a box, `pair(p.x, { p.x += 10; p.x })` of a
`Box<Point>`, `(11, 11)`. A write of a part of a local makes it a
variable, which nothing reads as alike (`count_place`); a write through
a reference is the write of what it points at, not of the local. That
asked MIR's `is_indirect`, which counts a box's deref too, so `*n.0 = ..`
and `(*p).x = ..` left the local alike. It asks `body_queries::indirect`
now, a deref of anything but a box, from the one rule `through_box` gives
of what a box's deref reaches. A borrow of such a part still asks
`is_indirect`: no read is moved past a borrow, and asking `indirect`
there made a `&*a.compute` of a `Box<dyn Fn>` a part of `a` that
`addr_eq` couldn't size. Results changed
([0262](0262-when-rust-and-js-disagree.md)), so Rust's order is kept.
One corpus case's JS holds what a variable's bounds check reads,
`const length = copy.rows.length;`, its `copy` now written through its
boxed slice (the user's decision, 2026-11-08). Held by the corpus case
`box_held_in_a_field_written`, the mutations `indirect-through-a-box`
and `count-place-written-is-indirect`. The rewritten case's other lines,
a reference held in a field set again, are #1034's.

## Amendment: a reference a field holds, set again (#1034)

The `field` rewrite's other lines put the reference itself in a field:
`node.0.v.set({ node.0 = &b; node.0.v.get() + 1 })` of a `Plain<'a>(&'a
Node)` set `b`'s where Rust sets `a`'s (native `11 10`, JS `10 11`), and
so did the reference set again through a box the field holds, `*held.0
= &d`, and the holder set again whole, `node = Plain(&b)`. The shared
borrow of a `Cell` asked only `through_local_reference`, a reference the
local is; a `&mut` already asked, of a field's reference, whether it's
given another (#988), by its exact place. Both branches now ask the same
two questions: `local_reference_written`, the reference the local is
set again, which holds the local where the borrow's made; and
`reached_through_written`, a reference a part of the local holds given
another, by an assignment of it or of what holds it (`held.0 =
Box::new(..)`), or lent to a closure that sets it, `|| node.0 = b`
(`written_or_lent`), the local set again whole, or an index set again, which
names the place as it is there (`place_where_made`, which now fixes a
handle's place too, the form a `Cell` a field is takes). Holding the
local can't serve the field: `node.0 = b` changes the array the local
holds in place. `read_before_written` is told what's written by one
rule, `writes_reach`: the place or what holds it, or an index it's at,
assigned or lent to be written; a `get` before the write still holds
nothing. Lent was missed of a variable that's the reference too:
`node.v.set({ let mut swap = || node = &b; swap(); 30 })` set `b`'s, as
only an assignment of `node` was counted, though borrowck lets a `&mut
node` be made while `(*node).v` is borrowed. Results changed
([0262](0262-when-rust-and-js-disagree.md)), so Rust's order is kept; no
other case's JS changed. Held by the corpus case
`reference_a_field_holds_set_again` and the mutations
`cell-through-a-field-reference-read-later`,
`field-reference-holder-set-whole-unseen`,
`field-reference-box-set-again-unseen`,
`place-where-made-handle-unfixed`, `written-or-lent-lends-nothing` and
`read-before-written-box-content-unwritten`; the net's two `field` lines
of #1034 run as native, and #1035's `given_last_unwinds`.

## Amendment: a reference an array's item holds, set again (#1075)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's. What #1034 made of a reference a field holds was of a field
alone: a write reached a place only where its projection was the start of
the place's, so `cur[0] = &mut b` didn't reach `(*cur[_5]).count`, MIR's
two reads of `cur[0]` each an index of its own temporary. A `&mut` held
in an array, `[&mut cur[0].count]`, then read `cur[0]` again where it was
used, `b`'s count where Rust's borrow is of `a`'s, `1 16` where native
prints `2 15`; and a `Cell` set through such an item while what it's set
to gives the item another, `node[0].v.set({ node[0] = &b; .. })`, set
`b`'s. The metamorphic net's `array` rewrite of five cases found it. An
item at an index of an array or a slice may be the one at any other now,
in the one answer of whether a write reaches a place
(`rvalues::within`), which `writes_reach` and `reference_written` both
ask. Held by the corpus case `reference_an_item_holds_set_again` against
native Rust; mutations `item-written-at-its-own-index-only`,
`item-reach-of-another-local` (`lower/mir/queries.ts`) and
`item-reference-written-unseen` (`lower/mir/rvalues.ts`).
