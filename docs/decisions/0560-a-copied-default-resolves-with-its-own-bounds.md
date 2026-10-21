# 0560. A copied default resolves a call with its own bounds

Status: Accepted. Extends [0541](0541-a-defaults-sized-is-its-impls.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a call in a
trait's default is the impl rustc picks for it.

## Context

aho-corasick's `Automaton::try_replace_all`, a default copied into each
impl (ADR 0049), indexes `replace_with: &[B]` by a `PatternID`, through
the crate's `impl<T> Index<PatternID> for [T]`. That was refused:
"calling `Index::index`". A call in a copied default resolved in the
impl's typing environment alone, which has no bounds of the default's
own type parameters, so `[B]: Index<PatternID>` found no impl. Where a
copied default's types are known, the impl's environment with the
default's own bounds, had one answer, `impl_typing_env` (ADR 0541), that
resolving a call and putting a type in the impl's terms didn't ask.

## Decision

**`impl_typing_env` is the one answer to where a copied default's types
are known: resolving a call there, `resolve_self_instance`, and a type in
the impl's terms, `impl_type`, both ask it.** The architecture test holds
that nothing else reads the impl's bare environment.

## Why

`default_crate_index` indexes a slice of a default's own type parameter
by the crate's `Index` impl, against native Rust; rustc's `traits/` tests
are unchanged.

## Consequences

- A mutation resolves in the impl's environment alone.
