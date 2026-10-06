# 0211. `Option::as_deref` of a `String` or a `Vec` is the option

Status: Accepted.

## Context

react.dev's `getRouteMeta` compares a route's path, `Option<String>`, with
`current_route.path.as_deref() == Some(path)`. rust-js didn't know
`as_deref`.

## Decision

**`as_deref` and `as_deref_mut` of an `Option<String>` or `Option<Vec<T>>`
are the option itself, as `as_ref` is (ADR 0023): a `&str` is the string a
`String` is, a slice the array a `Vec` is.** Of another type, whose
`Deref` may be one of the crate's, it's unsupported as before.

## Why

- **It's what's there**: the value doesn't change, only Rust's view of it.
- **It's tested**: a compiler test compares an `Option<String>`'s
  `as_deref` with `Some(..)` and takes a `Vec`'s first item through it.
