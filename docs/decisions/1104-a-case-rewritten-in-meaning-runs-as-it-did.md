# 1104. A case rewritten so its meaning is kept runs as Rust runs it

Status: Accepted (#939; the net the user's queue asked for, 2026-11-08).

Case ([0262](0262-when-rust-and-js-disagree.md)): none decided here. This
is a net: each wrong answer it finds is its own issue, decided there.

## Context

Lesson 2 of `docs/finding-wrong-answers.md`: context is part of the input.
Each corpus case is compiled in one shape, the one it's written in. The
lowering reads values by their shapes, a parameter's type, a generic
function's dictionaries, a value's place, and a shape the corpus never
wrote is one no case checks.

## Decision

**Each corpus case rewritten in ways that keep its meaning, each run
natively and as JS in both builds** (`scripts/metamorph`,
`test/metamorphic.ts`, `test/metamorphic.test.ts`):

- **The rewriter** is a small crate of its own, apart from the compiler's,
  over `syn`: it parses a case, rewrites it, and prints it. Its rewrites
  keep the meaning of what rustc still compiles:
  - `outline`: `main`'s body moved into a function `main` calls;
  - `boxed`: each `let x = e;` of a name bound once in its function made
    `Box::new(e)`, each use read through it, `(*x)`, a struct literal's
    shorthand field and a macro's tokens too;
  - `dyn-ref` and `impl-ref`: each reference parameter of a free function,
    `x: &T` or `x: &mut T`, taken as `&dyn MetamorphGet<T>` or `&impl ..`,
    and read back first, `let x: &T = MetamorphGet::metamorph_get(x);`,
    through an identity trait every `T` has; calls coerce, so they're as
    they were;
  - `impl-owned`: each owned parameter, `x: T`, taken as `impl
    MetamorphIs<T>` and read back first.
- **Native Rust's run of the rewrite is its oracle.** No snapshot is
  needed. A rewrite rustc rejects, a call that relied on a coercion a
  generic parameter doesn't make, is tried of its first place alone, and
  is no variant where rustc rejects that too.
- **What's compared**: each case of the corpus that runs, to its end or to
  a panic, on the host: a rust-js refusal is safe, and no failure; a crash
  of rust-js, or a run otherwise than native, is.
- **test/metamorphic-known.txt** lists each failing rewrite under its
  issue, `<builds> <rewrite> <case>`; the nightly runs each, so the list
  only shrinks.
- **Where it runs**: each push runs one case in 128, by a hash of its name,
  so a case added changes only whether it's sampled (as #853's groups);
  the nightly's `metamorphic` job runs every case, in four parts, about 4
  min on the Mac in 12.

## Alternatives

- **Text rewrites**: a regular expression can't tell a binding from a
  field or a path; `syn` parses what it rewrites.
- **Rewrites rustc's own MIR**: the lowering reads MIR, so a rewrite there
  would test what it's given, not the source shapes engineers write.
- **More rewrites at once**: each is one shape, so a failure says which.

## Consequences

Its first run, all 1,850 case-builds of each rewrite: `outline` found
nothing; `boxed` 26 cases, `dyn-ref` 16, `impl-ref` 14 and `impl-owned` 70
went wrong, nearly all in both builds. Among them: a type parameter's value
reached through a trait's method, not a parameter, stepped without its
dictionaries (`it.back is not a function`); a write through a `&mut dyn`
read back as `&mut T` lost. Each family is an issue, its cases listed.
`docs/finding-wrong-answers.md` lists the net; TESTS.md its row.
