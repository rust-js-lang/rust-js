# 0452. `{:?}` of a `PhantomData` of a type parameter shows the name its caller gives

Status: Accepted. Extends [0132](0132-std-odds.md) and
[0145](0145-type-facts.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same text,
rustc's name for the type.

## Context

`{:?}` of a `PhantomData<u8>` is `PhantomData<u8>`, its type's name as
`type_name` gives it (ADR 0132), but one of a type parameter, a derived
`Debug` of a generic struct's marker, `struct LiteMap<K, V, S> { values:
S, _key_type: PhantomData<K> }`, was refused: rust-js knew no name for
`K`. litemap, which url takes, stopped there. `type_name::<K>()` of a type
parameter is the name its caller gives, a type fact (ADR 0145), asked
where `type_name` is called; `{:?}` of a `PhantomData<K>` calls it inside
std, where the analysis that asks facts doesn't look.

## Decision

**`{:?}` of a `PhantomData<T>` is `PhantomData<` and `T`'s name, as
`type_name::<T>()` is: of a type parameter, the name its caller gives.
What `{:?}` of a type shows asks it, each `PhantomData<K>` in it or in a
struct of the crate's it holds, `recognition::phantom_named`: where it's
made a `dyn Debug`, a derived `Debug`'s field, and where it's given for a
`Debug` bound, `{:?}` written in place.**

```rust
fn marker<T>() -> String {
    format!("{:?}", PhantomData::<T>)
}
```

```js
function marker(TName) {
  return `PhantomData<${TName}>`;
}
```

## Why

- **It's std's text**: a generic struct's derived `Debug`, which shared
  code derives everywhere, prints its markers so.
- **One way to name a type parameter**: its caller's type fact, as
  `type_name` already is, through the analysis that hands facts out.
- **It's tested**: a corpus case formats a derived `Debug` of a struct of
  two markers, in place and through generic code, and a marker of a type
  parameter, against native Rust; a mutation of each part is caught.
