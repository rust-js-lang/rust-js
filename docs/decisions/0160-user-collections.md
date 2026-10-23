# 0160. A collection of the crate's: its own `IntoIterator`, `FromIterator`, `Extend`, `Sum` and `Product`

Status: Accepted. Extends [0049](0049-traits-and-generics.md),
[0055](0055-iterator.md), [0061](0061-generic-iterators.md) and
[0159](0159-user-from-str.md).

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A domain model's collections and amounts are types of the crate's: a cart
looped over, built by `collect()`, its prices added by `sum()`. Each impl,
`IntoIterator`, `FromIterator`, `Extend`, `Sum` and `Product`, was an
error, as an impl of a std trait rust-js doesn't call is (ADR 0049).

## Decision

**Each impl is the crate's function, called where Rust calls it:**

| Rust | JS |
|---|---|
| `for (name, price) in &cart` | `for (const [name, price] of cartIntoIterator_into_iter(cart))` |
| `let cart: Cart = items.collect()` | `cartFromIterator_from_iter(items)` |
| `cart.extend(more)` | `cartExtend_extend(cart, more)` |
| `let total: Money = prices.sum()` | `moneySum_sum(prices)` |

- **A std method that only calls its bound's is that bound's:** `sum()`
  is `S::sum(iter)`, `product()` `P::product(iter)`, `collect()`
  `B::from_iter(iter)`, `parse()` `F::from_str(s)`, read from the std
  method's own where-clause and resolved to the crate's impl, as `into()`
  is to `From`'s (ADR 0052). std's iterator is the bound's own type
  parameter.
- **A generic `I: IntoIterator`'s `into_iter()` is `I` itself:** a generic
  one is an array or a JS iterator (ADR 0061).
- **So a collection of the crate's can't be given where any
  `IntoIterator` goes,** to `v.extend(cart)` or to `fn f<I:
  IntoIterator>`: std's code and generic code iterate it as JS does, which
  never calls its `into_iter`. That's an error, saying so; `for` over it,
  and `cart.into_iter()` itself, are its own.
- `Extend`, `Sum` and `Product` have no diagnostic items: they're known by
  their names in `core`, as `FromStr` is (ADR 0159).

## Why

- **It's exact:** the `user_collections` corpus case compares a `Sum` of
  owned and borrowed items, `collect()` into a cart, `extend`, `for` over a
  `&Cart` and `into_iter()` of one with native Rust; diagnostics tests the
  refusals of a cart given to `extend` and to generic code. Without them,
  `v.extend(cart)` gave nothing and generic code threw.
- **It's the JS a person writes:** the type's functions, called by name.

## Amendment: through a reference bound

A collection of the crate's given where a bound on a reference goes,
`for<'a> &'a C: IntoIterator`, isn't refused: that bound is given its
dictionary, the collection's own `into_iter` (ADR 0457). One given where
an `I: IntoIterator` goes still is.

## Amendment: `B::from_iter(items)`

`Vec::from_iter(it)`, as itertools' `sorted` writes it, was refused for
any `B` but `String`, which had a lowering of its own. `B::from_iter`
is what `collect()` calls, so it's lowered as `items.collect::<B>()` is,
one way for every `B`: an array's, a `Vec`'s or a slice's items as they
are, another's iterator as `collect` steps it, then `collected_into`.
`Vec::from_iter([5, 4].into_iter().map(..))` is `[5, 4].map(..)`. Of
what's neither, an `Option` say, it's refused yet. The `from_iter_called`
corpus case makes each kind of collection so, against native Rust.

## Amendment: `from_iter` of a map or a set

`Vec::from_iter(set)` and the rest of a map's or a set's, which
`into_iter().collect()` is, were refused, as `from_iter` took only a
sequence or an iterator (#165). A map's or a set's are its items in its
order, `map_items`'s, as its `iter()` gives them: a B-tree's its keys'.
Held by `test/corpus/from_iter_of_maps.rs`.
