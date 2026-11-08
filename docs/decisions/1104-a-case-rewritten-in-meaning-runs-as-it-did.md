# 1104. A case rewritten so its meaning is kept runs as Rust runs it

Status: Accepted (#939; the net the user's queue asked for, 2026-11-08).
Amended (#1008, 2026-11-08): six more rewrites, a type parameter, a value
moved into an `Option`, an `Rc<RefCell>` and a struct's field, a closure
and a function each made the other, and the case's items moved into a
library crate, below.

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
  - `generic` (#1008): a free function's first concrete parameter type, a
    primitive's, `String`'s or the case's own type's, made a type
    parameter, `MetamorphT`, in its signature, bound by each trait of
    std's the type has that a body may ask (`Copy`, `Debug`, `Add<Output
    = MetamorphT>`, ..., a case's type by what it derives and each impl
    of it of no associated type), and each use of the function given the
    type, `f::<u32>(..)`, so it's the same function as generic code. It's
    the rewrite that found #1007 by hand. A body that names the type, a
    literal of it, `x + 1`, or one of its inherent methods, isn't
    generic code rustc compiles, so most of its places aren't variants:
    171 of 936.
  - `option`, `rc-refcell` and `field` (#1008): as `boxed`, each `let x =
    e;` moved into `Some(e)`, `Rc::new(RefCell::new(e))` and
    `MetamorphField(e)`, a one-field struct, and each use read where it's
    held, a place: `(*x.as_ref().unwrap())` (`as_mut` of a `mut` one),
    `(*x.borrow())` (`borrow_mut`) and `(x.0)`. A format string's own
    `{x}` is given as a named argument, `x = <read>`, so what it formats
    is the value, not what holds it. A move out of `x` reads it through
    a reference, which rustc rejects, so a moved value's rewrite is no
    variant. Two borrows of an `Rc<RefCell>` that overlap panic natively
    where the case's place never did: such a run, `RefCell already
    borrowed`, is no variant either (`overlapping`).
  - `closure-fn` (#1008): each closure that captures nothing made a `fn`
    nested in its function, `fn metamorph_closure_0(x: &i64) -> i64`,
    its types rustc's MIR's (`-Zunpretty=mir`, its signature of the
    closure, found by where the closure begins); one std's macros of
    expressions give, `println!("{:?}", v.map(|o| o.flatten()))`, too.
    And each free function given as a value, not called, made a closure
    that calls it, `map(|a0| double(a0))`. A case that compares functions
    or names their types, `fn_addr_eq`, `ptr::eq`, `type_name`, `as fn`,
    isn't rewritten: a closure isn't the function there.
  - `library` (#1008): every item but `main` moved into a library crate,
    `metamorph_lib`, each made public (its fields, an inherent impl's
    items, a `macro_rules!` exported), and the program `main`, which
    uses it, `use metamorph_lib::*;`. Native builds the library an rlib
    and the program with `--extern`; rust-js compiles it `--library`,
    and the program `--dependency` its manifest, in each build.
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
  the nightly's `metamorphic` job runs every case, in four parts. Of the
  eleven rewrites, 8 to 11 min on the Mac in 10 parts, a push's sample 16 s
  (#1008).
- **`METAMORPHIC_TALLY=<file>`**: a line of what each rewrite of each case
  made, changed or not, tried of its first place, a variant, for a run's
  counts.

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

Round 2's first run (#1008), every case of each new rewrite, in both
builds: the rewrites made 936 (`generic`), 1,708 (each of `option`,
`rc-refcell` and `field`), 748 (`closure-fn`) and 1,665 (`library`)
programs, of which 171, 1,361, 1,132, 1,696, 729 and 1,657 rustc
compiled; 2, 15, 11, 21, 1 and 97 of those ran otherwise than native or
crashed rust-js. `library` found the most: what a crate knows of its own
items, an iterator's own `size_hint`, a `Debug` impl called with `{:#?}`,
a static's value that changes, isn't known of another crate's.
Each family is an issue, #1010 to #1033, its cases listed under it.
