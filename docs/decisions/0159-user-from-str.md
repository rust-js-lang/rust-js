# 0159. A type's own `FromStr` is what `parse` calls

Status: Accepted. Extends [0049](0049-traits-and-generics.md) and
[0063](0063-text.md).

## Context

Shared models read their values from text, a form's field or a URL's
segment, through `FromStr`:

```rust
impl FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Role, String> { .. }
}

let role: Role = field.parse()?;
```

A user `impl FromStr` was an error, as an impl of a std trait rust-js
doesn't call is (ADR 0049). So was a `Result`'s `map_or` and
`map_or_else`.

## Decision

**A type's own `FromStr` is an impl like `From`'s: its `from_str` is a
function of the crate's, and `s.parse::<T>()` of the type calls it.**

| Rust | JS |
|---|---|
| `impl FromStr for Role { fn from_str(..) }` | `function roleFromStr_from_str(s) { .. }` |
| `s.parse::<Role>()`, `Role::from_str(s)` | `roleFromStr_from_str(s)` |
| `r.map_or(d, f)`, `r.map_or_else(g, f)` of a `Result` | `r.TAG === "Ok" ? f(r._0) : d`, `.. : g(r._0)` |

- **Its `Err` is the impl's,** any type: a `String`, an enum of the crate's
  with a `Display`, and `?` converts it as any error is.
- **std's `FromStr` has no diagnostic item,** so it's known by its path,
  `core`'s `str::FromStr`.
- **Still an error: `T: FromStr` in generic code,** whose `parse` needs a
  dictionary rust-js doesn't pass for it: `parse` to a `T`. (Amended: it's
  given one, ADR 0161.)

## Why

- **It's exact:** the `user_from_str` corpus case compares an enum's and a
  struct's `FromStr`, through `parse`, `from_str`, `collect()` and `?`,
  with native Rust; a diagnostics test the generic refusal.
- **It's the JS a person writes:** the type's parse function, called by
  name.
