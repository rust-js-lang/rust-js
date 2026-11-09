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
    and the program `--dependency` its manifest, in each build. A case
    that compares functions, as `closure-fn`'s list says, isn't
    rewritten either (#1027): a generic instance is one function across
    crates in no promise of Rust's (ADR 0395, amended).
- **Native Rust's run of the rewrite is its oracle.** No snapshot is
  needed. A rewrite rustc rejects, a call that relied on a coercion a
  generic parameter doesn't make, is tried of its first place alone, and
  is no variant where rustc rejects that too.
- **What's compared**: each case of the corpus that runs, to its end or to
  a panic, on the host: a rust-js refusal is safe, and no failure; a crash
  of rust-js, or a run otherwise than native, is.
- **test/metamorphic-known.txt** lists each failing rewrite under its
  issue, `<builds> <rewrite> <case>`; the nightly runs each, so the list
  only shrinks. A rewrite whose difference the user decided, listed in
  ADR 0262 and the guide's differences, is listed under that decision,
  `#rd<n>` (rd360's `refcell_borrow_while_mut`, #1032), not an issue's
  `#<n>`: no wrong answer to fix, and taken off where it runs as Rust does.
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

## Amended: the whole net, its runs kept by what they ran, and its native programs the machine's (#1047)

A fix the net found runs the whole net before it's pushed (#985), every
case's every rewrite: on the 12-core M6 in 12 parts, quiet, 830 s in a
fresh worktree (load 1.80), 501 s after a change to the compiler in a
worktree that had run it (load 3.87).

**Not taken: the cases the test map says a change reaches.** #1047 asked
for the net of only the cases whose compiles ran a function the change
touches (ADR 1101), each rewrite of each, and the cases the list names.
Built and measured before it landed, of the map of 1f0f4a45c: each of
the eight latest net-found fixes taken back out of the working tree
picks 1,880 to 1,883 of the 1,883 cases, or the map can't say of it, a
function added or a line between two (four of the eight). The place,
`&mut` and tidy's functions they change run in nearly every case's
compile. And it's unsound where it would save: #1016's `guard_value`
line alone picks 272 cases, but four of the five `rc-refcell` rows it
fixed aren't among them, as the rewrite borrows a `RefCell` in a case
that never did; a rewritten case runs what its rewrite adds. The whole
net stays the rule (main, on the user's behalf, 2026-11-09).

**What its time was, and what's kept now:**

- **Its runs, by what they ran** (ADR 0567): the rewrite's program, its
  JS, every file, native Rust's output and end, what every case's runs
  share and the net's own harness. A run the same as one that agreed
  before isn't run again; its compiles always are, as they're what's
  checked. After a fix, most rewrites' JS is as it was.
- **Its native programs, the machine's** (ADR 0104, amended): about
  14,000 of them, 7 GB, in a store each worktree used to hold to 3 GB, so
  a worktree's next check pruned most of them, and every task, in a
  worktree of its own, built them all again. And `library`'s were built
  again on every run, as their key held the library's `.rlib` by its
  time, which the run builds anew: now by what it holds.

On the M6, quiet, after a change to the compiler: 299 s (load 2.54), not
501 s; and in a fresh worktree, 295 s (load 2.67), no native program built again, not 830 s. A store's first run is
as before, 830 s (load 2.78).

## Amended: round 3, forms the corpus writes one way (#939)

Nine rewrites more, each of a form the corpus mostly writes one way,
which rustc holds to the same meaning:

- `array` and `tuple`: as `field`, each `let x = e;` held in an array of
  one, `[e]`, read `(x[0])`, and a tuple of one, `(e,)`, read `(x.0)`
  (`wrap.rs`).
- `for-each`: a `for` loop whose body leaves it only at its end, no
  `break`, `continue`, `return`, `?`, `.await` or label of its own, made
  `(e).into_iter().for_each(|x| body)`; and `for-loop`, back, a statement
  `e.for_each(|x| body)` of a closure of one untyped parameter that
  doesn't `return` made `for x in e body` (`loops.rs`).
- `if-let`: a `match` of two arms of no guards, one of which binds
  nothing, `None` or `_`, made `if let P = (e) { a } else { b }`: rustc
  held the `match` to cover every value, so the `else` is the other arm's.
  And a `let x = match e { P(v) => v, _ => d };` made `let P(x) = e else
  { d };`, a variant only where `d` diverges, as rustc refuses it otherwise
  (`matches.rs`).
- `method-fn`: an inherent method's body, of an impl of no generics, moved
  into a free function given its receiver, `self` named `this`, `Self` the
  type; and `fn-method`, back, a free function whose first parameter is a
  reference to one of the case's types made a method of that type the
  function calls (`methods.rs`).
- `default-copy`: each default method of a trait of no generics written
  out in each impl of it that takes it, `Self` the impl's type there as
  it is where the default runs (`defaults.rs`).
- `tuple-struct`: a struct of named fields that no other type of the case
  names alike, of no rust-js or serde attribute, made a tuple struct: each
  literal by its fields' places, its values first where it writes them out
  of order, so they're made in its order; each pattern by places, `_` of
  each it leaves out; each `.field` by its place, a macro's tokens too.
  Not a struct a literal of which takes another's rest, `..base`; nor any
  of a case with a binding, whose structs JS makes or reads by their keys,
  an object, where a tuple struct is an array (`tuples.rs`).

Its first run, every case of each, in both builds, 5 min 33 s on the
12-core M6 in 10 parts: the rewrites made 1,747 (`array`, `tuple`), 458
(`for-each`), 22 (`for-loop`), 212 (`if-let`), 87 (`method-fn`), 202
(`fn-method`), 64 (`default-copy`) and 622 (`tuple-struct`) programs, of
which 1,443, 1,735, 450, 22, 206, 85, 197, 61 and 619 rustc compiled; 5,
0, 5, 0, 0, 0, 0, 1 and 14 of those ran otherwise than native or crashed
rust-js. Four of `tuple-struct`'s were of a binding's struct, the
rewrite's own fault, so it leaves out such a case; the other 21 are ten
families, each listed under its issue in test/metamorphic-known.txt: a
`&mut` of a place through a reference an array's item holds, read again
where it's used; a write through `for_each`'s binding of `iter_mut()`'s
item not seen as a change in place, so `vec![row; n]` shares one row; a
closure's address read in another closure, `undefined`; a nested
closure's new value named as the variable it sets, hiding it; an iterator
`for_each` consumes not dropped where its closure panics; a generic
impl's method's drop of what an `IntoIterator` gives, in std's iterator
structs' build; an untagged enum's refusal read as a crash; a numbered
field's address in a tuple struct another holds; a generic tuple struct's
`RefCell` field made a cell and read as its value; and an untagged enum's
array member matching its `otherwise` member, a tuple struct.
