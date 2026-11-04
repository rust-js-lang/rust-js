---
name: Bug
about: A wrong answer, a crash, a refusal of valid Rust, or JS a person wouldn't write
title: ""
labels: []
---

<!--
Labels: one priority (CONTRIBUTING.md#issue-priority), one `kind: …`, one
`area: …`, and `regression` where a known commit broke what worked.
docs/bug-kinds.md defines them and asks for the facts below.
-->

## What happens

The Rust, the JS rust-js makes of it, and what native Rust does or what
the JS should be.

```rust
```

```js
```

## Facts

1. **Where the wrong JS is decided**, not where it shows (file, function,
   phase):
2. **The owner** of that question in docs/architecture.md's semantic
   owners, whether the faulty code asked it, and any other place that
   decides the same thing (file, function):
3. **Regression**: the first bad commit (bisected), `born:<SHA>` or `old`;
   and what kind of change it was (readability, a correctness fix #N, a
   feature):
4. **How it was found**: port (rd number), generated program (run and
   seed), mutation (name), corpus or CI test (name), probe or matrix
   (which), review, or external:
5. **The ADR 0262 case**, whether native comparison can see it, and if it
   can't, the JS-side test that holds it:
6. **The kind**, and one line answering its question:
