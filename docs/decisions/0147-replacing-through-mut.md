# 0147. Replacing a value whole through a `&mut`: an object in place, an enum by its place

Status: Accepted; amended (std's enums too, and an `Rc`'s place, below). Extends [0025](0025-vec-loops-refcell-mut.md),
[0033](0033-enums-with-fields.md) and [0099](0099-mut-references.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`*r = v` replaces what a `&mut` points at, and `mem::replace`, `mem::swap`
and `mem::take` do the same:

```rust
impl Counter {
    fn reset(&mut self) { *self = Counter::default(); }
}
impl Light {
    fn toggle(&mut self) {
        *self = match self { Light::Off => Light::On { level: 5 }, Light::On { .. } => Light::Off };
    }
}
```

A `&mut` to an object is the object (ADR 0025): JS has no pointer to a
variable, so `r = v` would only rebind the JS name `r`, and whoever owns
the object wouldn't see `v`. So each was an error, unless the `&mut` named a
variable's place (ADR 0099). Resets, state machines and `mem::take` of a
parameter are among what ordinary Rust does most with a `&mut`.

## Decision

**An object a `&mut` is becomes `v` in place; an enum with a fieldless
variant is reached through its place, as a number is.**

```js
const Counter = {
  reset(counter) {
    $assign(counter, { n: 0, label: "", items: [] });
  },
};
const Light = {
  toggle(self) {
    let tmp;
    if (self.value === "Off") {
      tmp = { TAG: "On", level: 5 };
    } else {
      tmp = "Off";
    }
    self.value = tmp;
  },
};
```

| Rust, of a `&mut` to an object that names no place | JS |
|---|---|
| `*r = v` | `$assign(r, v)` |
| `mem::replace(r, v)`, `mem::take(r)` | `$take(r, v)`: a copy of what `r` was, `r` becoming `v` |
| the same, what it was unused | `$assign(r, v)` |
| `mem::swap(a, b)` | `$exchange(a, b)` |

- **`$assign` makes the object `v`:** an array its items, a `Map` or a
  `Set` its entries, an object its fields, deleting those `v` hasn't, as an
  enum's variant changes them. Every name for the object sees `v`, as every
  path to Rust's place does.
- **An enum with a fieldless variant isn't an object for a `&mut`:** its
  `Off` is the string `"Off"` (ADR 0033), which can't become an object in
  place. Its `&mut` is what a number's is (ADR 0099): its variable's place,
  a box `{ value }` given to a function, a handle kept elsewhere; `*r = v`
  writes `r.value`, and a field through it is `l.value.level`. An enum each
  of whose variants has fields is always an object, and replaced in place.
  It's a user's enum this is true of, the crate's own or another rust-js
  crate's: one of std's or serde_json's, as `Value`, whose methods are
  rust-js's runtime's (ADR 0083), stays the object it was.
- **A type replaced whole is changed in place** (ADR 0052): a `Copy` value
  of it read into another variable is copied, `{ ...a }`, or `let b = a;
  reset(&mut b)` would reset `a` too. `*r = v` and `mem::swap`, `replace`
  and `take` of a `&mut` count, as `a.x = ..` does.
- **A place is still written as one:** `*r = v` of a `&mut` that names a
  variable's or a field's place, and `mem::take(&mut self.items)`, assign
  the place, `self.items = []`, as they did.

## Why

- **It's exact:** each name for the object sees `v`, as each path to the
  place does in Rust; the `replace_through_mut` corpus case compares
  resets, `mem::replace`, `mem::swap`, `mem::take`, a `Copy` value shared
  before it was reset, enums changing variant, a `Vec` and a tuple with
  native Rust.
- **It's the JS a person writes:** `$assign(counter, { .. })` where a
  person would assign each field, and an enum's place written as a
  variable is.

## Alternatives

- **Every `&mut` a handle, `{ get value() {..}, set value(v) {..} }`:**
  general, but each struct's `&mut self` method would read `self.value.n`,
  for the rare whole replacement.
- **A fieldless variant as an object, `{ TAG: "Off" }`:** it could be
  replaced in place, but every value of every such enum would be an object,
  where ReScript and ADR 0033 make it the readable string.

## Consequences

- `*r = v`, `mem::replace`, `mem::swap` and `mem::take` of a `&mut` to a
  struct, a tuple, a `Vec`, a map or an enum compile.
- An enum with a fieldless variant is still copied where it's read when
  another crate may change it (ADR 0100): its variants with fields are
  objects, though a `&mut` to it isn't.
- A string, a number, an `Option` or a `Box` replaced through a `&mut` is
  its place's, and isn't counted as changed in place: a string's `clone()`
  is still the string.
- An enum with a fieldless variant given by `&mut` to a function is boxed,
  as a number is: `toggle(x)` with `x = { value: l }`, then `l = x.value`.
- `$assign` copies `v`'s fields into the object: `v`'s own parts are
  shared, which is right, as `v` was moved in.

## Amendment: through a cell

**`mem::replace`, `mem::swap` and `mem::take` of a `&mut` generic code has
as a cell, a `&mut V` a method gives, is of its place, `cell.value`**,
written as a place is, the cell held once: litemap's `mem::replace(self.
get_mut(), value)` of its generic store, which was refused.

## Amendment: `clone_from`

**`dest.clone_from(&source)`, std's provided method, is `mem::replace` of
`dest` with a clone of `source`**, written as `mem::replace` writes, what
was there dropped, as `*dest = source.clone()` drops it: either's `Either`
forwards it. Of a generic `T`, the clone is its `Clone` dictionary's;
std's `clone_from` isn't asked of the dictionary, which has only `clone`,
but written in place, as std documents it's equivalent to. A crate's impl
known to have its own is called. A clone of an array made where it's
cloned, `$slice(v, 0, n)`'s, is that array (`clone_items`), as is one moved
into a collection, `[..].map(f).into()`.

## Amendment: a guard isn't written in place

A `&mut` to an object is the object, and `mem::swap` and `mem::replace`
of one write it in place, `$exchange(a, b)`. A `RefCell`'s or a lock's
guard is the cell it guards, so `mem::swap(&mut x.borrow_mut(), &mut
y.borrow_mut())` swapped what `x` and `y` held, where Rust swaps the
guards (#150). A guard isn't written in place (`written_in_place`):
two variables of guards swap as variables do, and a guard that isn't a
variable's is refused. Held by `test/corpus/guard_swap_refused.rs`.

## Amended: std's enums with a fieldless variant are by their place too

One of std's enums with a fieldless variant stayed its object, "its
methods' object", where the crate's was its place: `*r = Bound::Unbounded`
through a `&mut Bound<u32>` was `$assign(bound, "Unbounded")`, a
`TypeError`, as an object can't become a string. Found by the mutation
audit of this rule (`std-mixed-enum-place` survived: its test, of
serde_json's `Value`, had its own rule since, ADR 0083). **An enum with a
fieldless variant is by its place, std's as the crate's**
(`representation.rs`, `is_object`); one each of whose variants has fields
is its object, as before. `test/corpus/std_mixed_enum_mut.rs` runs it
against native Rust; the mutation `mixed-enum-object` holds both. Its JS
is the place's box, `const bound$1 = { value: bound }`, as the crate's is,
which the readability counts' renamed names and `return;`s grow by: the
price of the right answer (ADR 0262's case C).

## Amended: an `Rc` written through a `&mut` is by its place

React's update queue clones the next update and links it,
`let next = pending.next.borrow().clone(); *update.next.borrow_mut() =
next;`, of a `RefCell<Rc<Update>>`. The write through the guard's `&mut`
made `Rc<Update>` a type changed in place (`mutation::mutated_types`), so
its clone, which must then be a copy, was refused: "cloning
`Rc<Update>`".

**An `Rc` or a `Weak`, counted or not, is never changed in place by `*r =
v`**: its owners share what it points at, so a `&mut` to one is a cell,
a handle on its place (`is_boxable`, ADR 0523), whose write gives the
place another, `update.next = next`, `r.value = v`, and leaves what the
old one points at as it was. `object_like` says so, and a clone of one is
the one object, as any `Rc`'s is.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the place's write is
Rust's.

Held by `test/corpus/rc_assigned_through_mut.rs`, against native Rust: a
`&mut Rc` given to a function, a guard's write, `mem::swap` and
`mem::replace` of two, each with a clone of the old one read after; and
the mutation `rc-written-through-mut-changed-in-place`.

## Amendment: a `Vec`'s item written whole is its place (#349)

A slice's `v[i] = p` is the item's place written, `v[$at(v, i)] = p`; a
`Vec`'s, through `IndexMut::index_mut`, a `&mut` to the object, was the
object made `p`, `$assign($index(v, i), p)`. trekhleb/javascript-algorithms'
fastFourierTransform, ported, read so where upstream writes `output[signalId]
= upd1`. **A `Vec`'s `index_mut` written whole at once is the item's
place, as a slice's is**: `v[$at(v, i)] = p` (`bounds::index_mut_place`),
and `*r = p` of a `&mut` that's a place writes the place (`assign`).
At once allows the old value's drop between, `names[0] = first` of a
`Vec<Named>`, which can't change what the place is, borrowed as it is;
and its write where the drop unwinds isn't counted a use. ADR 0262's case
A: no other holder of the old object can see a difference, as Rust has
none. Held by `test/corpus/vec_item_written_whole.rs` and the mutations
`vec-item-written-in-place`, `vec-item-dropped-then-written-in-place`,
`written-at-once-cleanup-counted` and `place-ref-written-in-place`.

So is every `*r = v` whose `r` resolves to a known place, a field or an
item, `v[i + k] = p` of an index spilled first, and a guard's `value`,
`tmp.value = noisy`: the place written, never its old object made `v`.
`$assign` is left to a `&mut` that's the object alone, a parameter's.
Held also by the corpus's `lock_destructors`, `dyn_ref_cells`,
`generic_deref_mut` and `mut_assoc_through_dictionary`.

## Amendment: a swap of two places is JS's own (#369)

`std::mem::swap(&mut lower[0][r], &mut upper[r][c])` was `const t = ..; a
= b; b = t;`. **A swap of two places that read the same where they're read
again, an item's or a field's, is JS's own destructuring swap**, `[a, b]
= [b, a]`, as `v.swap(i, j)` is: its right side reads both, then each is
written in turn, what the three statements did. Not of two variables,
whose `let` tidy declares `const` where nothing else assigns them. Held by
the corpus's `swap_replace`, `slice_patterns` and `slice_pattern_mut_rest`
against native Rust, and the mutations `swap-of-places-held` and
`swap-of-variables-destructured` (`mir/entries.ts`).

## Amendment: an array taken is its items spliced out (2026-10-30, rd264)

`take()` of a `RefCell` of a `Vec`, which is the array (ADR 0863), and
`mem::take` of a `&mut Vec`, were `$take(queue, [])`: a copy, then the
array emptied in place. **An array taken is `queue.splice(0)`**, which
gives its items as a new array and leaves it empty, as `$take` did, with
no helper: React DOM's Fizz runtime's restore queue, moved into the
closure that restores it. One taken to another value, a struct's `{ n:
0 }`, is still `$take`. Case ([0262](0262-when-rust-and-js-disagree.md)):
none, the same array emptied and the same items given. `cell::taken` makes
both forms and `taken_parts` reads them, as a module variable's take does
(ADR 0270). Held by `lowering-cell-borrows.test.ts`'s "an array taken is
its items spliced out" and the mutation `taken-array-copied`
(`lower/std_types/cell.ts`).

