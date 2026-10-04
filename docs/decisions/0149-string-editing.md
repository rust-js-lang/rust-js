# 0149. A `String` changed in place is given a new string, by its byte offsets

Status: Accepted. Extends [0034](0034-strings-and-chars.md) and
[0138](0138-string-byte-counts.md).

## Context

A `String` is a JS string (ADR 0023), which can't change: `push_str` is
`s += t`, its place given a new one. `pop`, `insert`, `insert_str`,
`remove`, `truncate`, `retain`, `clear` and `with_capacity` were errors,
though they're how text is edited:

```rust
fn tidy(s: &mut String) -> Option<char> {
    s.retain(|c| !c.is_whitespace());
    s.insert_str(0, "[");
    s.push(']');
    s.pop()
}
```

## Decision

**Each gives the string's place the new string, as `push_str` does, and
counts by UTF-8 bytes, as Rust does (ADR 0138):**

```js
function tidy(s) {
  s.value = Array.from(s.value)
    .filter((c) => !/^\p{White_Space}$/u.test(c))
    .join("");
  s.value = $insertStr(s.value, 0, "[");
  s.value += "]";
  const popped = $strPop(s.value);
  s.value = popped[0];
  return popped[1];
}
```

| Rust | JS |
|---|---|
| `s.clear()` | `s = ""` |
| `s.truncate(n)` | `s = $strTruncate(s, n)` |
| `s.insert(at, c)`, `s.insert_str(at, t)` | `s = $insertStr(s, at, t)` |
| `s.retain(f)` | `s = Array.from(s).filter(f).join("")` |
| `s.pop()`, `s.remove(at)` | `const popped = $strPop(s); s = popped[0];`, and `popped[1]` |
| `String::with_capacity(n)` | `""`: the engine has its own |

- **Offsets are bytes, panics Rust's:** `insert` or `truncate` inside a
  `char` fails Rust's assertion, `self.is_char_boundary(idx)`; `remove` at
  the end says `cannot remove a char from the end of a string`, and past it
  or inside a `char` panics as slicing does.
- **`pop` takes a whole `char`:** an emoji's two UTF-16 units, and `None`
  of an empty string.
- **The place is written as `push_str`'s is:** a variable, a box of a
  `&mut String` (ADR 0074), a `Vec`'s item, `words[1] = ..`.

## Why

- **It's exact:** the `string_editing` corpus case compares each with
  native Rust on multi-byte text, an emoji, and through a `&mut String`;
  `string_insert_boundary` compares the panic.
- **It's the JS a person writes:** a new string assigned, as JS's own
  strings are edited.

## Consequences

- Each builds a new string, as `s += t` does: an edit is as costly as the
  string is long.
