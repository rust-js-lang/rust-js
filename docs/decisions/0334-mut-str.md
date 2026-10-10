# 0334. A `&mut str` is a cell of the text, written in place

Status: Accepted. Extends [0099](0099-mut-references.md),
[0138](0138-string-byte-counts.md) and [0149](0149-string-editing.md);
amends [0238](0238-kept-values-and-children.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Nothing wrote a `str` in place, so a `&mut str` was a `&str` where it was
made at all: `make_ascii_uppercase`, `as_mut_str`, `&mut s[a..b]`,
`get_mut`, `split_at_mut` and `from_utf8_mut` were errors, and so was
`strip_circumfix`, stable since Rust 1.98. A leaked `String`'s `&mut str`
was the string (ADR 0238), which its writes couldn't reach. `str` was 67
of 83.

## Decision

**A `&mut str` is a `&mut String` is: the place's own name where Rust's
borrow is, and a cell, a box or a handle, where it's kept or given (ADR
0099). A part of a string is a cell of those bytes of it.**

```rust
let mut name = String::from("élan vital");
name[2..4].make_ascii_uppercase();
if let Some(head) = word.get_mut(0..1) {
    head.make_ascii_uppercase();
}
```

```js
let name = "élan vital";
name = $asciiCase(name, true, 2, 4);
const head = $strGetMut({ get value() { return word; }, set value(value) { word = value; } }, 0, 1);
if (head != null) {
  head.value = $asciiCase(head.value, true);
}
```

| Rust | JS |
|---|---|
| `s.make_ascii_uppercase()`, of a `String`, a `Box<str>`, `as_mut_str()` | `s = $asciiCase(s, true)`, as `push_str` writes `s` (ADR 0149) |
| `s[a..b].make_ascii_lowercase()`, `slice_mut_unchecked(a, b)`'s | `s = $asciiCase(s, false, a, b)`: those bytes, checked as `&s[a..b]` is |
| `&mut s[a..b]` kept or given, `get_unchecked_mut(a..b)` | `$strPart(cell, a, b)`: checked as it's made, then read and written as those bytes |
| `s.get_mut(a..b)`, `split_at_mut(at)`, `split_at_mut_checked(at)` | `$strGetMut(cell, a, b)`, `$strSplitAtMut(cell, at)`, `undefined` where Rust's is `None` |
| `str::from_utf8_mut(bytes)`, `from_utf8_unchecked_mut` | `$fromUtf8Mut(bytes)`: `Ok` of a cell read and written through `bytes` |
| `get_unchecked(a..b)`, `slice_unchecked(a, b)` | `$strSlice(s, a, b)`, as `&s[a..b]` |
| `s.strip_circumfix(p, q)` of a `&str` or `char` pattern | `$stripCircumfix(s, p, q)`: `strip_prefix`'s, then `strip_suffix`'s |
| `s.leak()`, `Box::leak(b)` of one JS can't change in place | `{ value: s }`; `&*s.leak()`, read as a `&'static str`, is `s` |

- **A part stays where it is**: nothing a `&mut str` does makes it longer
  or shorter in bytes, so the byte offsets a part was made with stay right
  while the string around it changes.
- **`&mut *s.deref_mut()` is `s`**: a `String`'s `&mut str` given to a
  function, or written, is the `String`'s place, boxed and taken back
  after the call (ADR 0099), as a `&mut String` is. So is any std call
  that gives back what it's given.
- **A leak's `&mut` is a cell where it's one of a value JS can't change in
  place**, as each other `&mut` to one is: a function, a `Vec` and
  `map(String::leak)` may keep it, and writes through it are kept. A `let`
  holding one is named as its binding: `const r = { value: 5 }`. What's
  leaked is never dropped.
- **A binding is given a `Vec` of `&dyn Any`s as their values**, as a
  slice of them is (ADR 0331): react.dev's CodeBlock gives CodeMirror's
  `HighlightStyle.define` a `Vec<&dyn Any>` of specs.
- **Still refused**: `as_bytes_mut`, whose bytes would have to write the
  text back; `as_ptr`, `as_mut_ptr` and `substr_range`, which need where a
  string is in memory; and `strip_circumfix` of a closure or a set of
  `char`s, as `strip_prefix`'s is.

## Why

- **It's the same program**: each write changes the bytes Rust's does,
  and each range panics with Rust's message where Rust's does, as it's
  made.
- **It's the JS a person writes where it can be**: a `String` changed in
  place is its variable assigned, `s = $asciiCase(s, true)`, and only a
  part that's kept or given is a getter and setter.
- **It's tested**: the `str_mut` corpus case runs, against native Rust,
  `make_ascii_*` of a `String`, `as_mut_str()`, a `Box<str>`, a `Vec`'s
  item, a `&mut str` in a variable, a parameter and a field; a range
  written in place, from the start and to the end; a part given to a
  function and returned by one; `get_mut`, `split_at_mut`,
  `split_at_mut_checked`, the unchecked forms, and `from_utf8_mut` of valid
  and invalid bytes. `str_mut_boundary` and `str_part_boundary` panic
  inside a `char`, the second as the part is made. `leaked_string_written`
  writes leaks of a `String` and a number, kept, given and mapped, and
  never drops a leaked value; `str_circumfix` and `str_circumfix_closure`
  run and refuse `strip_circumfix`. A lowering test gives a binding a
  `Vec` and a slice of `&dyn Any`. Mutations edit all of a string for a
  range, run a range to the end, leave a part unchecked as it's made,
  write all of a string for a part, let `get_mut` and
  `split_at_mut_checked` panic, drop a box given to a function, keep a
  leak unboxed or its read boxed, and give a binding the pairs.
- `docs/std-coverage.txt`: `str` 79 of 83, `String` 32 of 43.
