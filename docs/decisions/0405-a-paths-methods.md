# 0405. A path's methods go over its components, as Rust's do on Unix

Status: Accepted. Amends [0173](0173-paths-as-text.md), extends
[0401](0401-a-path-compares-by-its-components.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what `parent()`,
`file_name()` and `join()` give is Rust's to the byte, `a/./b`'s parent
`a`, `.bashrc`'s extension none; where a separator in an extension
panics in Rust, it panics here.

## Context

A path is its text (ADR 0173), but its methods, `join`, `parent`,
`file_name`, `extension` and the rest, were refused, and so was changing
a `PathBuf`, `push` and `pop`: a `PathBuf` was taken for an object of its
fields. Build scripts and servers ported from Node read and make paths
all the time, as `path.join` and `path.dirname` do.

## Decision

**A path's methods are `path.js`'s, which go over its components as
Rust's `Components` does on Unix, a port of its state machine: what's
left of the path from its front and its back, so `parent()` is its slice
before its last component, trimmed as `as_path` trims it. A `PathBuf`
changed in place is its text's edit, as a `String`'s is.**

| Rust | JS |
|---|---|
| `p.parent()`, `p.file_name()`, `p.file_stem()`, `p.extension()` | `$pathParent(p)` and the rest, `undefined` for `None` |
| `base.join(p)`, `p.with_extension(e)`, `p.with_file_name(n)` | `$pathJoin(base, p)`, `$pathWithExtension(p, e)`, `$pathSetFileName(p, n)` |
| `p.starts_with(b)`, `p.ends_with(c)` | `$pathStartsWith(p, b)`, `$pathEndsWith(p, c)`: by whole components |
| `p.strip_prefix(b)` | `$pathStripPrefix(p, b)`: `Ok` of what's after, or `Err` of a `StripPrefixError`, `undefined`, shown as std shows it |
| `p.is_absolute()`, `p.has_root()`, `p.is_relative()` | `p.startsWith("/")`, or not |
| `p.iter()`, `p.ancestors()` | `$pathParts(p)`, `$pathAncestors(p)`: arrays made for them, which a `collect` needn't copy |
| `buf.push(p)`, `buf.set_file_name(n)` | `buf = $pathJoin(buf, p)` |
| `buf.pop()`, `buf.set_extension(e)` | `const popped = $pathPop(buf); buf = popped[0];`, and whether it did |

- **A component is `/`, a first `.`, `..`, or a name**, which is none of
  those: one string each, which `$pathParts`, `$pathEq` and `$pathCmp`
  read too (ADR 0401).
- **`is_absolute` and `has_root` are one question on Unix**, which has no
  prefixes, as Windows' `C:` is.
- **A `PathBuf`'s shape is a string's**, not its fields' object, so a
  `&mut` to one names its place, as a `String`'s does.
- **Still refused**: `components()`, whose items would be `Component`s,
  and what asks the file system, `exists()` and `canonicalize()`.

## Why

- **It's the program Rust runs**: the corpus's `path_methods.rs` asks
  each method of twenty paths, roots, `.`, `..`, doubled and trailing
  separators, hidden files and names ending in a `.`, joins, and pushes,
  pops and sets a `PathBuf`'s name and extension, goes over five paths'
  `iter()` and `ancestors()`, and strips prefixes, shows the error of one
  that isn't, each with native Rust;
  a runtime test panics on a separator in an extension, as std does.
  Mutations read a root's separator as a component's, keep a parent's
  trailing `.`, take `.bashrc`'s extension, append an absolute path,
  double a separator, and compare `starts_with` by text.
