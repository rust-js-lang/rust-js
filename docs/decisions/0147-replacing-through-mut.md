# 0147. Replacing a value whole through a `&mut`: an object in place, an enum by its place

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md),
[0033](0033-enums-with-fields.md) and [0099](0099-mut-references.md).

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
