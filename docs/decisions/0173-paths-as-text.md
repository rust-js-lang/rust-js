# 0173. A path is its text

Status: Accepted. Extends [0063](0063-text.md).

## Context

Error types name the file they're about, and thiserror shows one by its
`display()`:

```rust
impl<'a> AsDisplay<'a> for Path {
    type Target = path::Display<'a>;
    fn as_display(&'a self) -> Self::Target {
        self.display()
    }
}
```

`Path::display`, `PathBuf::from` and a path's deref were errors, which
stopped thiserror, and with it most crates' error types.

## Decision

**A `Path`, a `PathBuf`, what `display()` shows of one, an `OsStr` and an
`OsString` are the path's text, a JS string, as a `String` is (ADR 0063).**

| Rust | JS |
|---|---|
| `Path::new(s)`, `PathBuf::from(s)` of text | `s` |
| `PathBuf::new()` | `""` |
| `p.display()`, `p.to_str()`, `p.as_os_str()`, `buf.as_path()`, `&*buf` | `p` |
| `{}` of a `display()`, `{:?}` of a path | its text, and quoted, as a string's |
| `buf.clone()` | `buf`: nothing changes a string in place |

- **Not a string otherwise:** a path's `==` compares its components, so
  `a/b` and `a//b` are equal, which a string's wouldn't be. `==`, ordering
  and a path as a map's key stay errors, as do the path methods not here,
  `join` and `parent` among them.
- **Text in, text out:** a path is made only from text here, so it's
  always valid UTF-8, and `to_str()` is always `Some`.

## Why

- **It's exact:** the `path_text` corpus case makes paths of a `String`, a
  `&str` and nothing, shows them with `{}` and `{:?}`, reads them back with
  `to_str()`, clones one, and shows one through thiserror's `AsDisplay`
  shape, compared with native Rust; a diagnostics test the refusal of `==`.
- **It's the JS a person writes:** a path is a string in JS.
