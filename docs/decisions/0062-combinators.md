# 0062. Combinators and adapters: the closure's body in place

Status: Accepted. Extends [0030](0030-option.md), [0035](0035-results-and-throwing-js.md) and [0036](0036-iterators-and-sorting.md).

Case: C, A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Everyday Rust chains `Option`'s and `Result`'s combinators, iterator
adapters and `Vec`'s methods. rust-js had a few of each (`map`,
`unwrap_or`, `filter`, `fold`, `push`), and the rest were errors:
`unwrap_or_else`, `and_then`, `map_err`, `filter_map`, `zip`,
`max_by_key`, `contains`, `insert`, and so on.

## Decision

**Each is the JS a person writes for it, with a closure whose body is one
value written in place,** as `Option::map` already was:

| Rust | JS |
|---|---|
| `h.unwrap_or_else(\|\| 99)` | `h ?? 99` |
| `h.map_or(7, \|x\| x * 3)` | `h != null ? Math.imul(h, 3) >>> 0 : 7` |
| `h.and_then(half)`, `h.filter(\|&x\| x > 2)` | `h != null ? half(h) : undefined`, `h != null && h > 2 ? h : undefined` |
| `h.ok_or(e)` | `h != null ? { TAG: "Ok", _0: h } : { TAG: "Err", _0: e }` |
| `r.map(f)`, `r.map_err(f)`, `r.and_then(f)` | `r.TAG === "Ok" ? { TAG: "Ok", _0: .. } : r`, … |
| `v.iter().filter_map(f)` | `v.map(f).filter((item) => item != null)` |
| `flat_map`, `flatten`, `chain`, `zip` | `flatMap`, `flat`, `concat`, `$zip` |
| `take_while`, `skip_while`, `step_by(n)` | `$takeWhile`, `$skipWhile`, `.filter((_, i) => i % n === 0)` |
| `max_by_key(f)`, `min_by(f)` | `$maxBy(items, (a, b) => $cmp(key(a), key(b)))`, `$minBy(items, f)` |
| `product()`, `nth(n)`, `find_map(f)`, `partition(p)` | `reduce`, `items[n]`, `map(f).find(..)`, `$partition` |
| `v.contains(&x)` | `v.includes(x)`, or `v.some(..)` with `==` (ADR 0053) |
| `insert`, `remove`, `swap`, `truncate`, `dedup`, `extend` | `$insertAt`, `$removeAt`, `$swap`, `$truncate`, `$dedup`, `$extend` |
| `windows(n)`, `chunks(n)`, `concat()` | `$windows`, `$chunks`, `flat()` |

- **A `filter` of a variable goes into what takes its `Option` next**:
  `t.filter(|t| !t.is_empty()).unwrap_or("Error")` is `t != null &&
  t.length !== 0 ? t : "Error"`, as `title || 'Error'` is in react.dev's
  TypeScript, and `.map(|e| jsx! { <p>{e}</p> })` of one is `e != null &&
  e.length !== 0 ? <p>{e}</p> : undefined`: its test holds only where the
  variable isn't `None`, so it's what `unwrap_or` and `map` test, and the
  variable what they take, with no `const` of the `Option` between. A
  conditional of another `Option`, `c ? maybe() : undefined`, isn't one,
  as its `maybe()` may be `None`. (Amended.)
- **`flatten` of `Option`s is their `Some`s**, `.filter((item) => item !=
  null)`, through a reference too: `parts.iter().flatten()` of
  `&[Option<&str>]` was `parts.flat()`, which keeps `undefined`, so
  `["a", undefined].flat().join(" ")` was `"a "`. The corpus's
  `iter_flatten` runs it beside native Rust. (Amended.)
- **A closure made of statements gets a name first:** `const then = (x) =>
  { .. }; r.TAG === "Ok" ? then(r._0) : r`.
- **Closures inside the body don't stop it going in place,** unless they
  read the closure's parameter: `ok.then(|| items.iter().map(|n| n +
  1).collect())` is `ok ? items.map((n) => (n + 1) >>> 0) : undefined`. A
  closure that reads none of the names replaced is the same closure
  wherever it's made. One that reads `x` of `by.map(|x| ..)` still names
  the outer closure, as before: whether it runs at once is the method's,
  not the closure's, to say.
- **A field of what's there, or nothing, is an optional chain:**
  `named.and_then(|named| named.name.as_deref())` is `named?.name`, where
  it was `named != null ? named.name : undefined`. One property only, of
  the very path tested, with nothing as `undefined`: `a?.b.c` would end the
  chain at `.c` as well, and `map_or("none", ..)`'s fallback stays.
- **Rust's order is kept:**
  - an argument Rust computes whatever happens, like `map_or`'s default,
    goes in a `const` first if it has effects;
  - a closure only runs when Rust would run it, on the side of the `??` or
    `? :` that needs it.
- **What panics in Rust panics here:** `insert` past the end, `remove`
  and `swap` out of bounds, and `windows(0)`. `splice` would clamp
  instead.
- **On a lazy iterator (ADR 0055):** the adapters JS's iterator helpers
  share (`map`, `filter`, `flatMap`, `find`, `reduce`) stay lazy. `zip`,
  `chain`, `take_while` and `skip_while` of one are errors.
- **Not yet:** these on an `Option` of a generic `T`, which may be boxed
  (ADR 0051).

## Why

- **Checked against Rust's own output.** One `report()` formats every
  result with `{:?}` on both sides, and the strings must be equal; out of
  bounds must panic in both.
- **The common case reads as JS.** `h ?? 99` is what a person would
  write, not a call of a helper.

## Since

- **The path tested may be a key written out** (2026-10-10):
  `dict::get(files, "/src/styles.css").and_then(|f| js::get(*f, "visible"))`
  is `files["/src/styles.css"]?.visible`, as react.dev's SandpackRoot
  writes it, where it was `files["/src/styles.css"] != null ?
  files["/src/styles.css"].visible : undefined`: `x["k"]` is the same
  path as `x["k"]`, of the same text. A lowering test reads a property of
  an `Option` and of a dictionary's entry, and keeps a key that isn't a
  name, `file["a-b"]`, tested; a mutation tells no keys the same.
