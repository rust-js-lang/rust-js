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
  ADR 0262 and the guide's differences, is listed under that difference's
  number in the guide's list, `#difference <n>` (difference 39's
  `refcell_borrow_while_mut`, #1032), not an issue's `#<n>`: no wrong
  answer to fix, and taken off where it runs as Rust does (ADR 0262's
  amendment, an exception's name is its difference's number).
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

## Amendment: a night's net fits its machine's disk (2026-11-09, #1108)

The nightly's net ran its machine out of disk 80 minutes in, `ENOSPC`, and
its last 250 rewrites failed as they were written. Its machine had the
disk's floor free, 40 GB, so no native build was kept (`mayKeep`): each was
built in a fixture, which its test file holds till it ends, some 45,000 of
them, several MB each on Linux, std's debug info in each. **A build that
isn't kept goes once it's run** (`ranOnce`), and **a native program is
built without debug info** (`-Cstrip=debuginfo`), which no run reads, a
panic saying its location, a few hundred KB each. The nightly's net and
neighbour shuffle install Miri, so the reducer reduces a case with
`unsafe` code too (#1104). And the generator's `replace("", ..)`, which a
loop made of its string each time, its length multiplied, of eight chars
at most (#1106). Held by `test/programs.test.ts`'s "a native build that
isn't kept goes once it's run, and a kept one stays".

## Amendment: a case run natively on wasm32 is rewritten too (#1114)

A corpus case that says `//@ native: wasm32`, whose output is a `usize`'s
or a hasher's that wasm32 and rust-js agree on and the host doesn't, was
left out of the net. Its rewrites run natively on wasm32 too, as the case
does, the `library` rewrite's crate built for it (`runWasm32`'s `rustc`
flags). The object-model net, whose library crate is the host's, keeps to
the host's cases. The pointer cases of #1114, whose offsets are past a
`usize` borrow flag, are run through every rewrite so.

## Amendment: a change's own cases are rewritten each push (#1165)

A case a change adds or edits had its rewrites run only by the nightly's
whole net, one case in 128 by a push's: #1115's case ran as Rust does,
but not moved into a library, and that wrong answer, #1164, reached
`main` unseen until a net ran it hours later. Now each rewrite of each
corpus case the change adds or edits runs, in both builds: in
`check:fast`, plain and `--all`, of the working tree's and the branch's
since `main`, and in a job of each push's check, `metamorphic-cases`, of
what changed since `plan`'s `since`. `bun scripts/iteratorSlice.ts run
metamorphic [parts [since]]` lists them (`changedCases`, by their paths,
`test/corpus/<case>.rs`) for `RUST_JS_CASES=i/n:<file>`, which the net
reads as the corpus does: the cases a list names, every rewrite of each.
Two new cases took 9 s in 2 parts on the 12-core M6, cold (2026-11-10);
a change of no case runs nothing. `test/ci.test.ts` holds that a change
adding or editing a case runs its rewrites, and that a change of neither
doesn't.

## Amendment: the case made loud (#1203)

About half of a day's P0s (2026-11-10's review) were std's behaviour
rust-js writes in its runtime, drop orders, clone counts, a panic part
way, and the consumer net found 11 of them in a day as its types print
their drops and clones and it panics at each point. Most corpus cases are
quiet: a drop or a clone out of place prints nothing. Five rewrites make
each case loud (`loud.rs`):

- `loud`: each struct and enum of the case's own, in any scope, of no
  `Drop` and not `Copy`, given a `Drop` that prints, `~loud <n> drop
  <its Debug>` (a non-generic one that derives `Debug`) or `drop <Name>`;
  each derived `Clone` written out as `derive(Clone)` writes it, printing
  `clone ..` first, and each `Drop` and `Clone` of the case's own printing
  first too; each closure's call, `call <k>`, and each `for` loop's step,
  `step <k>`, a line. One counter numbers every event; past 2,000 they're
  counted, not printed. A `Drop` rustc rejects, of a type moved out of
  or one a `const fn` drops, is taken off the types its errors name, then
  off all, then the clones written out (rustc `--emit=metadata`).
- `loud-drop`, `loud-clone`, `loud-call` and `loud-step`: `loud` with a
  panic at the middle event of that kind in `loud`'s own native run
  (`METAMORPH_PANIC_AT`, `panicPoint`), where nothing is unwinding
  already, so the rest of the run is what the unwind drops. Of a case run
  natively on wasm32, which aborts on a panic, none is a variant.

Native Rust's run of each is the oracle, as of every rewrite: what the
JS drops, clones and calls, and in what order, is what it prints.

Its first run, every case of each in both builds, about 6½ min on the
12-core M6 in 12 parts (the part with the case below 15 min): the
rewrites made 1,684 (`loud`), 1,168 (`loud-drop`), 132 (`loud-clone`),
959 (`loud-call`) and 611 (`loud-step`) programs, of which 1,683, 1,157,
130, 953 and 605 rustc compiled. **None ran otherwise than native**:
the nets before it, the consumer net's loud iterators and the tables'
panic points, had found what a case's own drops, clones and unwinds
show. One crashed rust-js: `loud`'s `Drop` on `nested_generic_types.rs`'
innermost doubling type took two minutes to compile (#1214, ADR 0098
amended, fixed before this landed). And the rewriter's own: a closure's
body `unsafe { .. } as u32` written after the call's line read as a
statement; a body not a block is now in parentheses.

## Amendment: the case in other contexts (#1204)

The corpus writes each value in the context it was written for: a
closure literal where it's called, the concrete iterator a chain makes,
a crate that asks nothing of addresses, capacities or counts. The
lowering's rules decide on those contexts, and a user's code puts the
same values in others. Eleven rewrites give each case another:

- `opaque` (`opaque.rs`): each iterator the case makes, `v.iter()`,
  `it.map(f)`, given on through `metamorph_opaque::at_k(..)`, a function
  that returns it as an `impl Iterator` promising what it is of
  `DoubleEndedIterator`, `ExactSizeIterator`, `FusedIterator`, `Clone`
  and `Debug`. An opaque type is one rustc reveals only after analysis
  (#1142).
- `dyn-fn` (`dyn_fn.rs`): each closure called through a `Box<dyn Fn>`,
  `FnMut` or `FnOnce` as rustc's MIR says it's called (`closures.rs`'
  `Signature.kind`), given on through `metamorph_dyn::at_k(..)`.
- `fact-addresses`, `fact-slices`, `fact-stored`, `fact-capacity`,
  `fact-fn-identity`, `fact-chars`, `fact-write-fails`, `fact-rc-counts`
  and `fact-clones` (`facts.rs`): the case given a neighbour, never run or
  never called, that turns on one of the facts `AnalyzedCrate` decides for
  the whole crate, so what the case prints is its own.

A place rustc rejects, a `Peekable`'s `peek` through an opaque type, a
closure whose own type is needed, is left as it was: rustc's JSON errors of
the rewrite (`narrow.rs`) are mapped by their byte spans to the `at_k`
they're in, which is taken off, or for `opaque` given fewer promises (an
`E0277` naming one), and the program checked again.

Its first run, every case of each in both builds, 47 min on the 12-core
M6 in 11 parts, most of it reducing failures (DEVELOPMENT.md's slow
spots): `dyn-fn` made 788 programs, `opaque` 491, each fact about 2,000
(`fact-clones` 236). 68 failed, every one still wrong on main's compiler,
in eight families: `dyn-fn` 57, `opaque` 7, `fact-rc-counts` 4; every
other fact ran as native: a `Box<dyn Fn>` given to a std consumer never
dropped (#1220); one made a `dyn Fn` again, dropping twice or overflowing
the stack (#1221); a `&mut Box<dyn FnMut>` given as a `&mut dyn FnMut` a
place, not a function (#1222); `max_by_key` of a boxed key and a boxed
`filter` in generic code crashing rustc (#1223, #1224); `zip` calling a
boxed or hidden `map`'s function once more than std (#1225); a std
adapter behind `impl Trait` compiled from std's bodies and read as
rust-js's own iterator (#1226); and a counted `Rc` called through as one
not counted (#1227).

Two faults of the narrowing itself had left most of `opaque`'s cases
unrewritten, found as its harness test failed: an error where a value a
place made is used later, `let mut p = at_k(..).peekable(); p.peek()`,
mapped to no place, and the case was given up; and rustc says one unmet
bound twice, once of the call and once of its argument, so the second took
the whole place off where the first had taken its promise. An error that's
in no place and follows none is now the place made last before it's
written, and a promise is taken off however often it's named. `opaque`
then made 817 programs and `dyn-fn` 847 (135 s in 12 parts, without the
facts), and 56 failures more were listed: 44 more of #1226's, one more
of #1223's, an `eq` of an opaque iterator crashing rustc (#1229), and
`into_iter()` of what's already an iterator, given on as an `impl
Iterator`, giving nothing (#1230).

A case with a binding, `rust_js::link_name`, isn't rewritten by `dyn-fn`:
the boxed closure it hands JS is one Rust never drops (ADR 0873, the
guide's decided difference 12), which the case's native stub drops, so
its run differs by that decision, not by a fault.

## Amendment: generic code given other instances (#1237)

Most of what generic code decides is about every instance its callers
give it. The corpus mostly calls each consumer and adapter on the one
concrete iterator it makes, so a decision made of the generic type alone
was asked of only one instance. #1236 was found by hand, by giving
generic code many instances. `generic-instances` (`instances.rs`) does
this for every case: each `size_hint`, `count`, `last`, `nth`, `len`,
`max`, `min`, `collect`, `sum`, `product`, `take`, `skip`, `step_by` and
`rev` the case calls on an iterator it makes is called in generic code,
`metamorph_generic::at_k(..)`, `fn at_k<I: Iterator ..>(it: I) -> ..`. An
adapter gives an `impl Iterator` that promises what its instance keeps.
Each call is given one instance of the iterator: the case's own, a
`filter` that keeps every item, a `chain` of nothing, a `map` of each item
to itself, a `fuse`, or the net's own iterator that forwards `next`,
`next_back` and `size_hint`. Which instance a place gets turns with the
place and the case's length, so the corpus covers every instance of every
method. A place rustc rejects gets the case's own instance, then is left
as it was (`narrow.rs`). An iterator held in a variable isn't moved into
the call, because the variable may be used again. What the case does
with a value is done in generic code too (#1237's value instances): each
`x.clone()` is cloned by `fn at_k<T: Clone>(x: &T) -> T`, each `Some(x)`
made by `fn at_k<T>(x: T) -> Option<T>`, each `drop(x)` dropped by `fn
at_k<T>(x: T)`, and each `*r` given to a call, a macro or a `let` copied by
`fn at_k<T: Copy>(x: &T) -> T`, of whatever `T` the case has there, a
number, an object, a dropping type or an `Option`. An iterator's instance
turns with the iterators' places alone, so the value places leave it as it
was.

Its first run covered every case in both builds in 118 s, in 12 parts on
the 12-core M6 (2026-11-10). It made 639 programs, and 65 cases failed.
54 are #1239: generic code's `collect()` into a `B: FromIterator` is not
std's `Vec` collect. It never asks the hint, the capacity differs, and
the in-place and unwinding drops differ. Native monomorphizes
`B = Vec<T>`, while rust-js goes through the dictionary. The other 11
are #1240: `size_hint`, `count`, `last` and `nth` in generic code of
some instances. All are listed under their issues.

The value instances' first run, with every other rewrite, `METAMORPHIC=all`
over every case in both builds, 2,022 cases in 45 min in 12 parts on the
12-core M6 (2026-11-10), found 7 cases wrong under four issues: a `Clone`
dictionary of an `Option<Option<T>>` that declares its parameter's name
again (#1246), a `()` copied through a `&mut ()` a struct holds (#1247), a
nullable parameter given a variable a generic `Some` was assigned to
(#1248), and a closure cloned in generic code that is the same function,
`Rc::ptr_eq` of an `Rc<dyn Fn>` of each `true` (#1249). Each is listed
under its issue.

## Amendment: a rewrite native Rust doesn't end is no variant (#1252)

`generic-instances` can make a program native Rust never ends:
`repeat_count_panics.rs`' `repeat(L(3)).skip(2)` given on through an
identity `map` is a `Map`, whose `count()` folds the endless `Repeat`
where std's `Repeat::count` panics. Its native run and the JS's were each
cut, past 10 s or 16 MB of output, and compared, a verdict that flipped
with timing (`range_own_steps.rs` too). A native run stopped by the
harness is no program's whole answer: `execute` marks it `stopped`
(`test/programs.ts`), and a rewrite whose native run is is no variant, as
one rustc rejects or one that aborts on wasm32 is; the net tries its first
place alone, then none. Its two rows are off the known list. ADR 0262's
case: none, the compiler's JS is unchanged.

## Amended (2026-11-12, #1109): a case may keep each place's cost

`generic-instances` gives each place an instance in turn, starting at the
case's source length modulo the instances. A `map`, a `filter` or the net's
own iterator answers `nth`, `skip`, `step_by`, `count`, `last`, `min` and
`max` by stepping each item it passes. That's Rust's own semantics:
`Map::nth` calls its function on each item. Where the case's own iterator
answers at once, a range's `skip(4_000_000_000)` does, native Rust finishes
only because its optimizer folds the loop away, and the JS runs past its
10 s. So `range_reversed_skipped_beside_its_back_stepped` failed or passed
by its length.

**A case whose point is what its iterators skip at once says so,
`//@ instances-keep-cost: <why>`** (`test/corpus-cases.ts`, the one reading
of the directives). Its places get the case's own instance wherever the
turn's would step each item (`keeps_its_cost`, `scripts/metamorph`).
`METAMORPH_KEEP_COST` carries the directive to the rewriter. Every other
case turns through every instance as before, so `count()` of a `filter`
still runs. Held by the harness test "generic-instances keeps each place's
cost of a case that says its point is what it skips at once", and by the
range case, which no longer pads its comment to pick its instances. ADR
0262's case: none, the compiler's JS is unchanged.

## Amendment (2026-11-12): a program std leaves to a seed has more than one right answer

The `loud` rewrite of `std_keys_around_their_own_hash.rs` failed main's
check about half the time, and the nightly's loud triage's class (b)
with it. Its native run drops a hash map's values in the order of std's
`RandomState` seed: run ten times, it printed `drop Version(2)`, `drop
Version(1)` five times and the reverse five times, where the JS drops
them in insertion order (the guide's difference 5). Both are Rust's
answers. Now, where the JS's run disagrees with native Rust's, native
Rust is run again, up to eight times (`nativeTries`). The rewrite runs
as Rust does if one run agrees (`againAgrees`). Native output that
doesn't vary still disagrees every time, so a wrong answer is found as
before; of two orders a seed picks, one is missed 1 time in 256.

## Amendment (2026-11-13, #1453): a hash table's drops, compared as a set

Eight runs again don't cover a hash table of four: `value_keys.rs`'s loud
rewrite drops a map's four keys, 24 orders, so its insertion order was
one of native's runs about 30% of the time, and the nightly flaked on it
either listed or not. Now, where no run again agrees, all of native's
runs, its first and those again, are compared with the JS's up to a hash
table's order (`agreesUpToHashOrder`): the lines some run puts other than
the first does are the seed's, and each block of them in a row is taken
for a hash table's iteration only where every line of it, of each run and
the JS's, is a loud drop; the JS's block is then the same drops, in any
order, each loud count in its place. Every other line, standard error and
the outcome are compared as before. So a drop that native's runs all put
in one place, one no hash table orders, still disagrees: a compiler that
drops a struct's two fields in reverse (`drop_ladder`'s `.rev()`, run by
hand against a program with a four-key map) was rejected over native's 7
orders in 9 runs, and the compiler as it is accepted. A hash table's
iteration that prints, not drops, isn't reordered. Held by
`metamorphic.test.ts`' five tests of it; `value_keys.rs` ran as Rust does
three times in a row.

Amended (2026-11-13, nightly 43333093940): each drop, clone or call is a
group, its loud line and the lines it prints up to the next loud line, so
a key whose `Drop` prints is compared as one whose doesn't; within a block
native's own runs order differently, any loud line may start a group, the
counts in place, a constant line after the block's last group compared
where it is, and a drop given another's lines never agrees. Where native is
deterministic the order is compared exactly: two calls no hash table
orders, swapped, still disagree (the lead's bounds). A hash table whose
order changes what the program prints, not only its order, `stock_orders`'
runs of different lengths, is still compared exactly.

