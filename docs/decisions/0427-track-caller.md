# 0427. `#[track_caller]`: a function that reads where it was called is given it

Status: Accepted.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): where a call is, a
`Location`'s file, line and column, is what a program reads, so it's Rust's,
paid for in a parameter.

## Context

A `#[track_caller]` function is given where it was called, which
`Location::caller()` reads: a helper that panics, or logs, names its
caller's line rather than its own. std's `Index::index` is one, so every
`v[i]` of a type of the crate's calls one.

```rust
#[track_caller]
fn here() -> &'static Location<'static> {
    Location::caller()
}
let at = here();                      // line 41, column 14
```

`Location::caller()` was an error, so was rustc's
`indexing-spans-caller-location`, and three more of its tests.

## Decision

**A function that tracks its caller, and reads where it was called, takes
it: `location`, before its evidence, and each call gives it.**

```js
function here(location) {
  return location;
}
const at = here({ file: "test/corpus/track_caller.rs", line: 41, column: 14 });
```

- **A location is `{ file, line, column }`,** where rustc's
  `span_as_caller_location` puts a call: at the outermost macro's call,
  its file as rustc names it, its column counted from 1. `{}` of one is
  `file:line:column`, and `{:?}` `Location { file: "..", line: ..,
  column: .. }`, as std's; of one written where it's shown, the text itself.
- **Which functions take it is `analysis::caller_locations`'s**, once for
  the crate: one that tracks its caller, `#[track_caller]` or its trait's
  declaration's, as rustc's `codegen_fn_attrs` says, and reads
  `Location::caller()` in its own body, or calls one that takes it, or
  raises a panic itself (amended, #2350). One that reads and raises
  nothing of it takes none, so `v[i]` of an `Index` of the crate's that
  raises nothing is the call it was. A library's manifest says it of each item it
  exports, `takes_location`, which its consumers' calls give. Every
  function's parameters and calls ask `takes_caller_location`, the one
  question.
- **A call gives its own location, or, in a function that takes one, the
  one it was given,** as rustc passes it on.
- **`Location::caller()` is the given location in the body of a function
  that takes one, and where it's written anywhere else,** a closure in it
  too: the closure's body is its own, `body_owner`, which a closure lowered
  from MIR now says too, as one from THIR did.
- **A tracked function as a value**, `.map(here)` or `here as fn()`, is an
  arrow that calls it with its definition's location, as rustc's shim of
  it does.
- **Through a dictionary, generic code's or a `dyn`'s** (amended), a
  method one of whose impls, or its default, takes its caller's location
  takes it in each entry of it, after its arguments, and each call through
  one gives it, as rustc's calls through a vtable and monomorphized generic
  code do. Where its declaration tracks its caller, every call gives it;
  where only an impl does, generic code's gives it, as rustc's direct call
  of the impl does, and a `dyn`'s none, which that impl's entry has as its
  own definition's, `location = { .. }`, as rustc's shim gives it. An entry
  passes it on where its impl takes it.

  ```js
  only_impl: (arg0, location = { file: "src/lib.rs", line: 35, column: 5 }) =>
    unitTracked_only_impl(arg0, location),
  ```
- **A shim gives its definition's location** even inside a function that
  tracks its caller: a function pointer's arrow and a dictionary's entry,
  `location_at` of its definition, where `caller_location` would be what
  that function was given.
- **Not yet:** a tracked trait method as a value, and a library's trait
  method without a default whose entries take the location, which its
  manifest, of functions, can't say: errors.

## Why

- **It's exact:** the `track_caller` corpus case reads a location directly,
  passed on, in a closure, through a function pointer, in a `map`, and
  shown with `{}` and `{:?}`, with native Rust. The corpus leaves the
  repository's path out of the files either names, so a case's location is
  the same on any machine. The `track_caller` pair of crates calls a
  library's from its consumer, through the manifest.
- **It's what a person writes where it matters, and nothing where it
  doesn't:** the parameter is there only where something reads it.

## Alternatives

- **Every function that tracks its caller takes one.** It's what rustc
  does, and every `v[i]` of the crate's would carry an object of where it
  is that nothing reads.
- **A location as a string, `file:line:column`.** Its parts would be parsed
  back out, and a file with a colon in it would be wrong.

## Consequences

- rustc's `tracked-trait-impls` and `tracked-trait-obj`, of tracked trait
  methods called through a dictionary, pass (amended); the
  `track_caller_traits` corpus case runs each kind of call through a `dyn`,
  generic code and a copied default against native Rust, and the
  `track_caller` pair a library's tracked default through its consumer's
  generic code. `file-is-nul-terminated` reads a file's C string, which is
  addressable memory, and `track-caller-ffi` calls through FFI.


## Amendment: a `Location` is its text, and every panic raises one (2026-11-22, #2292)

A `Location` is the string `"file:line:col"`, no longer an object of its
parts: `{}` is the text, `{:?}` std's `Location { file: "..", line: N,
column: N }`, written as it compiles where the text is known and else by
`$locationDebug`; `file()`, `line()` and `column()` read it from its end,
as a file may hold a `:`; `==` and its order compare its parts. And every
helper that raises a panic takes where it's raised, `raisedAt`, the
caller's line, the lowering's location of the MIR statement or call it
makes it in, `caller_location`'s, or a site of std's own, `$stdSites`
(ADR 1184).

## Amendment: a function made a value is given the location of the shim it's called through (2026-11-23, #2335)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior is kept.

rustc calls a `#[track_caller]` function made a value through a shim, and
the shim's location is the one it's given. Measured natively:
- through `FnMut::call_mut` (`iter.map(Option::unwrap)`, an `F: FnMut`): `core/src/ops/function.rs:166:5`;
- through `FnOnce::call_once` (`Option::map`, `F: FnOnce`, `Box<dyn FnOnce>`): `250:5`;
- through `Fn::call` (`F: Fn`, `&dyn Fn`): `79:5`;
- as a `fn` pointer: the function's own definition, `core/src/option.rs:1013:5` of `Option::unwrap`.

The JS gave a std function the line of the call it was given to, and the
crate's own its definition's whatever the shim.

**Each function made a value is told how it's called (`mir/shims.rs`'s
`through`), from every use of it in the body:**
- a call's argument, by its parameter's bounds, the most they name of `Fn`, `FnMut` and `FnOnce` (`generic_facts.rs`' `called_through`; `F: Fn`'s supertraits make `Fn` the most);
- a cast to a `fn` pointer;
- a `dyn` of one of them;
- a local it's put in, by each use of the local, `let f = Option::unwrap; v.map(f)`.

It's then given its shim's location: the trait's method, or its own
definition for a pointer. The crate's own function gets it as its location
argument (`value_location`), and std's helper as where its panic was raised
(`raised_at`). A location in std's own files is written as the shipped std
names them, `$stdSites.source + "/library/core/src/ops/function.rs:166:5"`.
`$stdSites.source` is the toolchain's `/rustc/<commit>`, written by
`bun run std`, so the JS is the same on every machine (`location_as_shipped`;
#2343's lesson: this compile reads std from where this machine keeps it).

**Undecided, and kept as before until Tu decides (refuse in binaries, or a
documented exception):**
- the uses disagree;
- a use is none of the above: stored in a struct or captured;
- a `&dyn Fn` of a borrow MIR makes a promoted constant, whose cast is in the body that holds it.

Each such corpus case carries `located-otherwise: #2335`.
`std_function_value_as_a_dyn_panic.rs` is one. (Decided since, below.)

**Found on the way: a wrong answer, fixed.** A function value's call was
given the facts of the MIR call it's an argument of. In
`Option::map(nested, Option::unwrap)`, `nested` is present, so the
`unwrap` checked nothing and gave `undefined`, and a `fn` pointer to it gave
`undefined` for `None`. Such a call is now made apart (`called_apart`), with
no facts of the call around it.

Held by corpus cases: `std_function_value_panic.rs`, `_once.rs`,
`_fn.rs`, `std_function_pointer_panic.rs` and
`std_function_value_in_a_local_panic.rs`, each against native's location.
`own_track_caller_value_panic.rs` waits on #2350, the crate's own
`#[track_caller]` function naming its own line.

## Amendment: a tracked function that raises a panic itself takes the location (2026-11-23, #2350)

rustc places every panic a `#[track_caller]` function raises itself where
it was called, as each of its tracked calls is given that location: a
`panic!`, whose `panic_fmt` tracks its caller, an `unwrap()`, a `Vec`'s
`v[i]`, and the function's own `Assert`s, a division's by zero or a
slice's bound. Every one of them is raised at `caller_location`, the
given location where there is one; but only `Location::caller()` and a
call of one that takes it made a function take it, so one that only
panicked was given none and named its own line, `main.rs:4:9` where native
Rust names its caller's, `main.rs:10:20`. `caller_locations` now asks of
each tracking function, by its MIR, which a std body alone has (ADR 0561),
or else its THIR, whether it raises one itself: an `Assert`, or a call of
std's that tracks its caller, neither the crate's compiled here nor a
library's, whose taking it is found as theirs is. So a crate's `Index`
impl of a `Vec`, whose trait tracks its caller, takes it, and `grid[7]`
names its line, as natively; one that raises nothing, `quiet`'s unchecked
`x + 1`, or calls only such a one, takes none, still the parameter only
where something reads it. The THIR scan saw a function's coercion to a
pointer, `let f: fn() = here`, as a call; its MIR doesn't, so a function
that only calls through one no longer takes a location it never read.
Std's compiled `Repeat::last` and `count` take it too, which #2324 gives
them where they're called.

A trait's method without a default whose declaration tracks its caller
takes the location in every crate alike, as rustc's every impl of it
does: whether one of its impls reads it can't be what decides, as one
crate's generic call is given another's dictionary, and a library's
manifest, of functions, can't say it of a declaration without a body.
So wasm-bindgen's `UnwrapThrowExt::expect_throw`, whose impls now take it
as they call `expect`, isn't refused as a library's method its manifest
can't speak of, and std's `Index::index` is given it by every crate's
generic code; an entry whose impl reads none passes it on to nothing. A
default, whose body decides, takes it as that body does; an impl that
tracks its caller where its declaration doesn't is still refused in a
library, as before (`std_types::location::tracked_without_default`).

## Amendment: std's impls that track their caller, through a dictionary and a `Repeat` (2026-11-23, #2324)

Where a trait's declaration doesn't track its caller but std's impls of it
do, and rust-js's model of them raises a panic there, generic code's call
gives its location, as rustc's monomorphized call of the impl does:
integers' `Div::div` and `Rem::rem` and their `/=` and `%=`, integers' and a
`char`'s `Ord::clamp`, and `SliceIndex::index` and `index_mut`. That's one
owner's answer, `recognition::std_raises_at_caller`: such a method takes
the location in every crate alike (`std_types::location::takes_location`),
not only in one whose generic code calls it, as a library's generic
`clamp` is given its app's dictionary, which took none and was handed the
location for its drop (the corpus's library rewrite of
`own_ne_max_min_clamp`),
std's operator entries take it after their arguments and pass it to their
helper, `div: (a, b, location) => $div(a, b, location)`, and an `Ord`'s and
a `SliceIndex`'s dictionaries pass it on. `Ord::clamp`'s default doesn't
track its caller, so a type on it, a `String`'s, names std's own assert,
`$stdSites.ordClamp`, called directly or through a dictionary. A method
whose model raises nothing, `Add::add`, which wraps, takes none.

**A helper's location given only where its call asks.** The one parameter
is `raisedAt`, and the runtime's own parameter list says how it's filled
(`runtime::location_at`, `runtime::location_where_asked`): written
`raisedAt`, `Expr::call` gives it at every call, as before; written
`raisedAt = undefined`, never, but where the call says it may be needed,
`Expr::call_located(callee, args, asked)`, a missing optional argument
before it `undefined`. `$count`, `$last` and `$lastDropping`, whose panic
only a `Repeat`'s own `count` or `last` raises, are asked where `recognition::may_be_repeat`
says the receiver may be one: a type holding a `Repeat`, or a type
parameter's, a projection's, an opaque type's or a `dyn`'s, so a concrete
other's `count` is `$count(it)` as it was. The location reaches the
`Repeat`'s own method through any adapter that passes the call on: held
while `$count` or `$last` runs, in a slot of `globalThis` under a
`Symbol.for` key of `runtime/marks.js`, which every copy of the runtime
reads, and put back however the call ends, `$unwinds`'s on a panic.


A library's function made a value at a shim was its module's one
`const`, `hereValue`, made with its first use's location, so a `Fn` use
after a `fn` pointer's said where it's defined (found by the survivor
`library-located-fn-value-a-call-arrow`). Only a value given where it's
defined is the module's one function now; one given a shim's location
elsewhere is an arrow of its own, as a function item a trait calls has no
pointer whose identity anything sees. The pair
library_singles_across_crates holds both, and that two `fn` pointers to
it are the same function.

## Amendment: what the uses don't say is refused in a binary (2026-11-23, #2335 follow-up, #2355)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior is kept; where it can't be told, a compile error (Tu's decision:
"try harder, then refuse", no documented exception).

**More is decided (`mir/shims.rs`), each measured natively:**
- **A shared reference to the value**, `&f`, reborrowed `&*r`, or a
  promoted `&Option::unwrap`, whose constant `Found` now matches through
  the promoted body's `referred_constant`. Every `Fn*` impl of `&F` calls
  `F::call`, so it's called through `Fn::call` where it's made a `&dyn _`
  or given to a parameter that's a type parameter bound by a `Fn` trait,
  whatever that bound: `79:5`. A reference given to a `&F` parameter
  stays untold, as generic code may copy `*r` out and call it through
  `FnMut`. **A wrong answer, found and fixed:** a reference was asked by
  the bound of what it was given to, so `apply(&f)` of an `F: FnMut`
  named `call_mut`'s line, `166:5`, where native names `79:5` (#2355).
- **A box of it**, `Box::new(f)`: the box's uses. Made a
  `Box<dyn FnOnce>`, whose one impl calls `call_once`: `250:5`. Read
  through, `*b`: the value, by its uses. Its drop calls nothing.
- **The unsizes that name one way whatever follows** are the only ones
  decided: `&dyn _` and `Box<dyn FnOnce>`. A `Box<dyn Fn>` is called as its
  own calls say, natively `79:5` called itself, `166:5` given to an
  `F: FnMut`, `250:5` to an `F: FnOnce`; a `Box<dyn FnMut>` and a
  `&mut dyn` too. These stay untold.
- **A local called itself**, `f(x)`, is a call of the function at its own
  line, through no shim, as native's: it says nothing of the value, so
  `let f = Option::unwrap; v.map(f); f(None)` is decided by `map`.
- **Returned**, the value's local `_0`, or made where no use is seen, is
  untold, where it was taken as never called.

**`through` gives `Called`: `Through(way)`, `Never` (no use reads it), or
`Untold`.** An untold value of the crate's own body is refused where a
panic's location is written: `recognition::prints_panic_locations`, a
binary's compile, `--crate-type bin`, not a test build, whose `$main` alone
writes std's hook's text (ADR 1184; `process.rs`'s `entry_of` asks it too).
The error goes through `refusal.rs`'s one way, `Refused::Instead`, with
what to write: ``rust-js does not support a value of `Option::unwrap`, a
`#[track_caller]` function, called where this function doesn't say how
(#2335) yet``, help ``where it's called says the location its panic
names: give it straight to the function that calls it, or write a
closure, `|x| Option::unwrap(x)`, whose line is the location, as in
Rust`` (the closure's parameters by the function's arity). A library, and
a test build, keep compiling it at its caller's line.

The corpus and the metamorphic net compile a case as a library, so the net
asks a variant that panicked as native did but elsewhere, all it wrote
alike, whether it compiled as a binary is refused (`refusedWhereLocated`):
then it's a refusal, as any variant rust-js refuses. Its five container
rewrites listed under #2335 are refused so, and the `boxed` one is decided;
test/known/metamorphic/2335.txt is gone.

Held by corpus cases against native's location,
`std_function_value_as_a_dyn_panic.rs` (its `located-otherwise` off),
`_boxed_once_panic.rs`, `_in_a_box_panic.rs`, `_borrowed_panic.rs` and
`_called_itself_panic.rs`, which test/binaries.test.ts also runs as
binaries; and its refusals, a local used two ways, a `Box<dyn Fn>`, a
value returned, captured or in a struct given whole, and the crate's own,
each refused in a binary and compiled in a library. The guide's
limitations page lists it, its example compiled as a binary.

**A library a binary links says so** (Tu's decision kept: libraries
compile, and no binary writes a wrong location). A library's untold value
is recorded as its function's lowering finds it (`Dependencies`'
`untold_locations`, by `FnCx::untold_location`: the value as written,
where, and its closure) and listed in its manifest,
`Library.untold_locations` (library ABI 47; a closed-world fact whose
consumer that differs is refused, docs/library-contract.md). A binary that
links a library listing any is refused before it's lowered, naming each,
``rust-js does not support a library whose function makes a value of
`Option::unwrap`, a `#[track_caller]` function, called where it doesn't
say how, `tracked`'s at src/lib.rs:2:13, in a binary, which writes where a
panic was raised (#2335) yet``, with the same help
(`location.rs`' `refuse_untold_locations`, asking `prints_panic_locations`;
one wording, its `refuse_untold`). Of the library at all, however
much of it the binary calls: a value the library hands out, a `dyn Fn` or
a `fn` pointer, is called where no call of the binary's names its
function, so a list per function, closed over its callers, would miss
those. Measured: none of the crate corpus's 150 compiled crates, none of
React's port's crates, and not `@rust-js/std`, lists one; the binding
crates, `js`, `webapi` and `react`, are checked by rustc alone and make no
such value.


## Amendment: what a generic call gives is what its instances may track (2026-11-24, #2457)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named another
line than Rust's.

A `RefCell`'s `Clone::clone` is `#[track_caller]` in std's impl, though `Clone`'s declaration isn't, as #2324 found of
integers' `Div`, `Rem` and `clamp`; through generic code's dictionary its
panic named where the dictionary was made, `$borrow(value,
"main.rs:5:16")`, where native names `x.clone()` in the generic function.
And #2324's rule was a hand list, per method: every entry of a listed
method took a location, the crate's own impls' too, and every generic call
gave one. Of `Clone`, that would be every entry and every generic clone.

The rule has two halves now. rust-js's: which of its models raise at
their caller, `recognition::model_raises_at_caller`, integers' `/` and
`%`, `clamp`, a `RefCell`'s `clone`, whose `clone_from` generic code runs
as `*dest = source.clone()`, its clone's, as std's borrows its source
there too; not `Add`, which
std's impl tracks but rust-js's wraps, as a release build does (ADR 0011).
rustc's: whether the impl it resolves tracks its caller where the
declaration doesn't, its attribute (`impl_raises_at_caller`).
`SliceIndex::index` leaves the list: its declaration tracks its caller
itself, so #2350's rule gives it, and slice.rs asks `takes_caller_location`.

It's per instance. An entry takes the location only where its impl
tracks (`entry_takes_location`), `{ clone: (value, location) =>
$borrow(value, location).slice() }`; a `Vec`'s and the crate's own take
none. A call through a dictionary gives it (`model_location`) only where
an instance that tracks may reach it (`analysis::Located::gives_location`):
of callers it sees, `instances`'s, the one owner of what generic code's
type parameters may be (ADR 1115); of callers it can't all see, a `pub`
function's, a dictionary's impl parameters, past 64, where a std impl
that tracks may be the call's `Self` by the owner's own bounds
(`tracking_impl_may_be`, an inference probe of each impl's type against
every bound), so `T: Iterator + Clone`, never a `RefCell`, gives none
whoever calls it. That's sound across crates: no crate downstream may
write an impl of another crate's trait for a `RefCell`, which isn't its
own, so the owner's environment sees every impl that may make a bound hold;
through `&T` or `Box<T>`, fundamental, a self type still has to be local,
which `&RefCell<Local>` isn't; only a bound whose other arguments are
unknowns too, `T: Into<U>`, may be met downstream, so it may hold. A call
that gives more after the location, `clamp`'s drop, says so
(`location_followed`): each entry takes the location's place, and the
call gives `undefined` where nothing that tracks reaches; another model
method given evidence after its location is refused where it's called.
A `#[track_caller]` function that calls one through a dictionary takes
its own caller's location, which it gives on (`caller_locations`).

Moved: 13 corpus cases, each a generic clone given its location where its
`T: Clone` may be a `RefCell`, a `clamp`'s `undefined`, and `Div`'s
entries losing their unused parameter; std's `T: Clone`-only impls'
clones, `Option`'s, `Rev`'s, `Cycle::new`'s, which native names std's
line of. Held by test/binaries.test.ts's rows against native, the two
clones, #2324's operators, a `#[track_caller]` function dividing
generically, and a library whose bound a `RefCell` meets through its own
impl and whose `I: Iterator + Clone` gives none, and the mutations
`model-clone-unraised`, `impl-tracking-unread`, `tracking-bounds-unread`,
`location-unfollowed`, `located-entry-untaken`, `generic-clone-unlocated`,
`dictionary-call-unlocated`,
`operator-entry-unlocated`, `tracked-fn-dividing-generically-untaken` and
`gives-location-never`, with #2324's retargeted.

## Amendment: a container's clone raises its item's panic where its clone clones it (2026-11-24, #2471)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named another
line than Rust's.

A container whose clone clones a `RefCell`, a `RefCell`'s clone tracking its
caller, raises its panic where the container's own clone impl clones the
item: std's line of std's containers, `option.rs:2280:31` of `Option`'s
derive, `result.rs:1898:27` of `Ok`'s and `1899:29` of `Err`'s,
`slice.rs:432:38` of a slice's `to_vec` (a `Vec`'s, a `BinaryHeap`'s, a
`Box<[T]>`'s), `clone/uninit.rs:20:33` of a `Box`'s, `array/mod.rs:494:21`
of an array's, and of rustc's shims, `clone.rs:236:5` of a tuple's
`Clone::clone` and `ops/function.rs:166:5` of `FnMut::call_mut`, a
`VecDeque`'s and a `LinkedList`'s `iter().cloned()`; and the derive of a
crate's own struct or enum. rust-js named the line the clone was called at.

Each of std's is read from std's MIR by `bun run std`, as `$stdSites`
(`compiled_std/raised.rs`): its walk counts a call of a method whose model
raises at its caller, a derive's `x.clone()`; follows a closure held in
another it gives, an array's in `try_from_fn`'s `wrap_mut_1`, and a trait
declared in a function, `to_vec_in`'s `ConvertVec`, to its impls; takes a
shim's site where its method is declared (`Owner::Declared`). The lowering clones each container's items under
its site (`clone_site`), a `Result`'s each variant's, a crate's derive its
impl's span; one inside another raises at the innermost's, as Rust's does.

Held by test/binaries.test.ts's rows against native (each container, a
`Result`'s `Err`, `Option<Vec<RefCell>>` nested, a crate's derive alone and
in `Some`), test/std-sites.test.ts, and the mutations
`raised-model-calls-unread`, `raised-declared-walked`,
`raised-nested-trait-unresolved`, `raised-held-closures-unfollowed`,
`container-clone-unsited`,
`result-variant-unsited` and `derive-unsited`.

## Amendment: std's `Index` entry raises where generic code indexes (2026-11-24, #2488)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named
another line than Rust's.

`Index::index` and `IndexMut::index_mut` are `#[track_caller]` where the
trait declares them, so every impl's panic is raised at its caller: of
`&items[i]` in generic code over an `S: Index<usize>`, the generic body's
own index. Generic code's call already gave its site, but std's entries of
a slice, an array, a `Vec` and a `VecDeque` were arrows made where the
dictionary was, `(items, i) => $index(items, i, "<there>")`, which named
the caller's line. Each now takes the location it's given,
`(items, i, location) => $index(items, i, location)`, the check made under
`Expr::raised_at(location)`. Its entries the same wherever they're made,
a body's dictionary of one is named once and given on. Held by the corpus
case `index_in_generic_code_names_the_body` and the mutation
`std-index-entry-located-elsewhere`.

## Amendment: a capacity overflow std raises at its own line (2026-11-24, #2488)

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

The runtime raised every `capacity overflow` at raw_vec's line, which is
right where a `Vec` asks room for too many bytes, but not where std's own
code panics before asking: a slice's `repeat` of a count past
`usize::MAX`, its `checked_mul(n).expect("capacity overflow")`,
`slice.rs:526:50`; and a collect of `0..=usize::MAX`, whose hint has no
upper bound, std's `TrustedLen` collect, `spec_from_iter_nested.rs:57:18`.
`$stdSites` gains `sliceRepeat`, read from std's MIR, and `$repeatItems`
and `$range` raise those at their own rows before asking the room. Held
by the corpus cases `slice_repeat_past_usize_panics_at_its_checked_mul`
and `collect_of_a_range_past_usize_panics_at_trusted_len`, and the
mutations `slice-repeat-at-raw-vec`, `range-past-usize-at-raw-vec` and
`slice-repeat-row-of-another`.

## Amendment: a `Repeat`'s step reached through an adapter names the adapter's line (2026-11-24, #2488)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named
another line than Rust's.

A `Repeat`'s own `count` and `last` panic, "iterator is infinite", naming
their caller, as std's track it (#2324). Reached through an adapter, its
caller is the adapter's method, which doesn't track its own, so Rust names
the nearest frame that doesn't, the innermost adapter's call: `Skip::count`'s
`skip.rs:78:19`, `Enumerate::count`'s, a chain's each side's, a flatten's
inner's, `Copied`'s, a sized `Box`'s `last` (`boxed/iter.rs:57:17`), a
chain's `last`, which gives `Iterator::last` to `Option::and_then`, the
`FnOnce` shim's (`function.rs:250:5`). rust-js named the outermost call's.

The runtime holds the caller's location while a `count` or `last` runs
(`$infinitelyAt`, restored as it ends, however it ends); each runtime
adapter that passes the step on gives its own row's site, `$count(src,
drop, $stdSites.skipCount[0])`, so the innermost wins, as Rust's nearest
frame, and a `peekable()`'s arm, known only as it runs, is its own. The
rows are std's MIR's (`compiled_std/raised.rs`'s `HELD_STEPS`: an adapter
method's calls of the same step of what it holds; `Owner::Resolved`, the
impl rustc picks for a type over a `Repeat`, `Box`'s sized `BoxIter`,
`FlattenCompat`'s general one), held to std's step catalog both ways by
`bun run std` (`held_steps_checked`). A `fuse()`'s and a `peekable()`'s,
std's compiled adapters, give std's own line already. Of a frame JS
compiles away, a `Box`'s or a `copied()`'s, the call's location is the
frame's row, chosen at its terminator (`repeat_frame_row`).

What's left: such a frame below a runtime adapter, `repeat(&x).copied()
.skip(1).count()`, names the adapter's line, as nothing of it stands
between them in JS (filed apart, its cases `located-otherwise`). Once
std's compiled adapters are the default (#1458), their bodies give these
sites themselves, and the rows serve only the in-place forms; #1458 keeps
them in step. Held by eighteen corpus cases, one per site, the nested one
and one that an adapter's site never names a later panic, the unit test
`held_steps_are_the_catalogs`, and the mutations of each row's use.

## Amendment: a std type's clone site is read of std's MIR, not a list (2026-11-24, #2509)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named the
caller's line where Rust names std's.

#2471 read a container's clone site from rows of eight types it listed; a
`HashMap`'s and a `BTreeMap`'s clone of a borrowed `RefCell` named the
caller's line. The rule is wider: wherever std's clone of a type calls a
`RefCell`'s `clone`, the frame nearest it that doesn't track its caller.
`bun run std` now walks std's monomorphic MIR from each std `Clone` impl
(`raised_through`), each type parameter that may be a `RefCell` (its
impl's clauses may hold of one) a `RefCell<u8>`, another its type's
default or a `u8`, through each call that holds it, resolved as rustc does,
specialized and through shims, at codegen's frame: through an inlined
function that doesn't track its caller, that function's line,
`option::IntoIter`'s `Option::clone`. It writes `clonedAt[type][position]`,
91 types. Of the five whose walk finds several sites, `VecDeque`'s
`try_fold` and `fold`, a `BTreeMap`'s leaf and internal nodes, a native
oracle settles it (test/std-cloned-at.rs, `cloned_at_oracle.txt`), and
`bun run std` refuses one it has none of. The lowering's `clone_site`
resolves the clone's impl and reads its row, `$stdSites.clonedAt[key]
[position]`, by the same owner (`cloned_at_site`), a map's key and value
each its own position, a std enum's variant its field's; one where no
`RefCell` may be needs none, and a std impl with no row where one may be
is refused. A `BTreeMap`'s entry in an internal node names the leaf's line,
the guide's difference 65, Tu's decision. #2471's container rows are gone.
Held by the corpus cases of a `HashMap`, a `BTreeMap` and a `VecDeque`,
`btree_map_clone_of_an_inner_entry_names_the_leaf_line` (`located-otherwise`
by difference 65), test/binaries.test.ts' container rows,
test/std-cloned-at.test.ts, the architecture test that `clone_site` names
no std type, and the mutations `clone-site-of-std-unread`,
`map-parts-unsited`, `cloned-at-inlined-frames-skipped`,
`cloned-at-oracle-unread`, `cloned-at-defaults-unused` and
`cloned-at-key-shapes-unsaid`. The item-cloning models, a `cloned()`'s, an
`extend_from_slice`'s and an `Option<&T>`'s `cloned`, follow.

## Amendment: an item a std method clones is placed at std's line of that method (2026-11-24, #2509)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named the
caller's line where Rust names std's.

A `cloned()`, an `Option<&T>`'s `cloned` and a `Vec`'s
`extend_from_slice` clone each item by `T::clone` handed to a `Fn*`
trait, so a borrowed `RefCell` item's clone, which tracks its caller,
names the shim's line, `function.rs:166:5` (`FnMut::call_mut`) or `250:5`
(`FnOnce::call_once`), or `Cloned`'s `try_fold` closure's,
`cloned.rs:29:32`. Which one depends on the step that runs: a `count()`
and a `fold` reach `166:5`, a `next()` and an `nth` `250:5`, an `any` the
closure. `bun run std` walks each of these methods from std's MIR by the
same walk (`cloned_methods`), each step of the catalog's `STEP_NAMES`
of a `Cloned` of a `slice::Iter` of a `RefCell<u8>`, and writes a
`clonedAt` row for each, keyed by its path,
`core::iter::adapters::cloned::Cloned::count`. The native oracle settles
`nth`, `max_by` and `min_by`, whose walks find two paths.

The lowering places a `cloned()`'s item clone by the steps `back_stepped`
says its value is asked (`made_asked_steps`), and `cloned_items` (clones.rs)
is the one owner of that choice, which #1458's form will read. Where every
asked step's row is the same line, the clone is placed there; where no
asked step clones an item, nothing is placed. Where the steps aren't
known, or are of two lines, the clone keeps the call's line: a `cloned()`
collected, whose native line follows its source's `TrustedLen` (`166:5`
of a slice's, `250:5` of a `filter()`'s), a `for` loop's, one returned,
stored or handed to generic code, a `zip`'s (std's
`__iterator_get_unchecked`, `cloned.rs:70:55`), or a `next()` then a
`count()`. JS can't tell a `collect` from a `for` loop: both step
`next()`. A runtime form that chooses by the step that runs would guess
wrong, and a refusal would reject common code for a line, so these keep
the call's line, `located-otherwise` against #2509 (Tu's decision), until
std's `Cloned` and `collect` are compiled (#1458), which places them by
construction. An `Option<&T>`'s `cloned` and `extend_from_slice` each have
one line and are always placed. The metamorphic net's rewrites that move
the iterator into a `Box`, an `Option`, a field, an array or a tuple, or
hand it to generic code, are listed in test/known/metamorphic/2509.txt
until `back_stepped` follows a value into a container (#1458).

Built of std's compiled `Cloned` (`--std-iterators`, #1458's path), a
`count()`'s, a `fold`'s and an `any`'s clone name `std/lib.rs:1:1` and
`<unknown>`: the compiled `FnMut::call_mut` shim and `try_fold` closure
pass no line of their own. That is #2522, a P0 to fix before #1458 can
close #2509's cases by construction; the three cases are in
test/iterator-forms.test.ts' `differs`, and their rewrites in
test/known/metamorphic/2522.txt.

`back_stepped` read a named variable's debug info as a use, so any
iterator held in a `let` was asked every step. It now reads it as no use
(`steps.rs`). The one corpus move is `dyn_wrapper_hides_the_crates_own_steps`.
A flatten of `Box<dyn Iterator>`s there, asked only `next` and `count`,
is JS's own `flatMap` instead of `$flatMap`: the `dyn` hides the crate's
own `count`, so std's `FlattenCompat::count` asks nothing JS's form
doesn't.

Held by the corpus cases `cloned_count_`, `cloned_fold_`,
`cloned_next_`, `cloned_nth_`, `cloned_any_`, `option_cloned_` and
`extend_from_slice_of_a_borrowed_refcell_names_std_line`, and the
`located-otherwise` cases `cloned_collect_`, `cloned_for_loop_` and
`cloned_next_then_count_of_a_borrowed_refcell_names_its_call`. They are
also held by test/std-cloned-at.test.ts' three new rows, and by the
mutations `cloned-items-row-unread`, `cloned-items-mixed-guessed`, `steps-debug-info-read`,
`option-cloned-row-unread`,
`extend-from-slice-row-unread` and `cloned-methods-backs-unwalked`.

## Amendment: each std item the catalog binds that clones what it's given is placed at std's line, the rows walked of the catalog (2026-11-25, #2577)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named the
caller's line where Rust names std's.

The amendment above walked a hand list, `cloned_methods`: `Cloned`'s
steps, an `Option<&T>`'s `cloned` and `Vec::extend_from_slice`. Nine
more of the runtime's models clone what they're given by `T::clone`, and
named their caller's line of a borrowed `RefCell`: `Vec::resize`
(`vec/mod.rs:3687:39`), `vec![x; n]` (`from_elem`, the same line), a
slice's `fill` (`clone_from`, `slice/specialize.rs:12:20`) and `to_vec`
(`alloc/src/slice.rs:432:38`), `Vec::extend_from_within`
(`vec/mod.rs:3766:45`), `VecDeque::resize` and `iter::repeat_n`
(`repeat_n.rs:125:37`), `iter::repeat` (`repeat.rs:83:27` of `next`, `101`
of `nth`), and `<[MaybeUninit<T>]>::write_clone_of_slice`
(`mem/maybe_uninit.rs:1242:41`). A hand list goes stale as models are
added, and this one had.

- The rows are the catalog's ([1202](1202-one-catalog-of-the-runtimes-models.md)).
  `bun run std` walks each std item a model or a form binds
  (`models::bound_paths`) that clones what it's given: a function or a
  method whose type parameter `Clone` bounds, its row by its path; an
  iterator whose `Iterator` impl does, a row of each step of std's
  catalog, by its path and the step's name. Each parameter is made of a
  `RefCell<u8>` as the clauses take it (`of_cells`): one `Clone` bounds the
  cell, an `Iterator` of references a slice's iterator of cells, one the
  impl's self type gives a parameter of its own with a default that
  default (`Vec`'s `A = Global`), another `()`. A model bound to its std
  item has its row by construction, and the rest are #2513's measure,
  ratcheted.
- The catalog binds the nine, and four of the lowering's forms that
  clone what std's code is given: `cloned`, `extendFromSlice`,
  `optionCloned` and `toVec`. `models::resolved` reads a slice's own
  method, `core::slice::<[T]>::fill`, of the impls of a slice in its
  crate. `path_of` gives the path of a std item a call is of, of the
  methods a path names each of its name, `Option<&T>`'s and
  `Option<&mut T>`'s `cloned`.
- One reader. MIR's call (`mir_call`) tells the lowering the std item
  it's of while it lowers it (`FnCx::calling`), and the first clone made
  under it, std's of what it's given, reads that item's row
  (`call_clone_site`): its path's, or, of a call that makes an iterator,
  its struct's of each step asked of what it makes where they're one
  line. A clone that clone makes places its own (`clone_site`): `resize`
  of `Vec`s of a cell names the `Vec`'s clone's line. The three readers
  that named their rows by text, `extend_from_slice`'s, `Option`'s and
  `cloned()`'s, read it so; `cloned_items` and its `ClonedItems` are gone.
  `Option<&T>::cloned`'s row's key is its path,
  `core::option::Option::cloned`, the one snapshot that moves.
- A step's clone is given its step's site. A `repeat`'s steps clone at
  four lines and a `take` asks several, so the steps asked don't say
  one. The runtime knows each step it runs: `$repeating` and
  `$repeatingN` give each clone the site of their step's row,
  `clone(value, $stdSites.clonedAt["core::iter::Repeat::nth"][0])`, and a
  clone of the iterator itself its derived `Clone`'s row. The lowering
  writes such a clone of a cell `(value, at) => ..`, its clone raised at
  `at` (`stepped_clone_fn`). It's the one form of a clone a step makes,
  which `cloned()`'s steps can take up without a second one. A clone
  that needs no site ignores it. test/std-sites.test.ts holds each row
  the runtime reads by its key to one the walk writes.

This is exact where the step that runs is known, as Tu's decision of
#2509 asks, so no exception is added. A `cloned()`'s clone, which the
lowering writes as a map, not the runtime, still keeps the call's line
where its asked steps are several or unknown, as above, until #1458.

A borrowed bare `RefCell` is moved into most of these only with its
guard leaked (`mem::forget`, `RefMut::leak`), as the cases do; a
`to_vec` and a `write_clone_of_slice` of a borrowed one are common.

Held by the corpus cases `vec_resize_`, `vec_from_elem_`, `slice_fill_`,
`slice_to_vec_`, `vec_extend_from_within_`, `vec_deque_resize_`,
`repeat_n_`, `write_clone_of_slice_`, `option_mut_cloned_`,
`repeat_next_`, `repeat_nth_`, `repeat_take_next_back_`,
`repeat_take_collect_`, `repeat_clone_`, `repeat_n_nth_`,
`repeat_n_next_back_`, `repeat_n_nth_back_`, `repeat_n_clone_` of a
borrowed `RefCell` naming std's line, and
`vec_resize_of_vecs_of_a_borrowed_refcell_names_the_inner_clones_line`,
each against native. The mutations `cloned-models-clone-nothing`,
`cloned-models-allocator-unit`, `models-slice-method-unresolved`,
`models-path-of-one-impl`, `call-clone-site-kept`, `call-clone-site-unread`,
`stepped-clone-reads-the-call`, `stepped-clone-unsited`,
`mir-call-callee-untold`, `repeat-nth-at-next` and `repeat-clone-at-a-step`
are caught. `cloned-items-at-unplaced` goes with the code it mutated: a
`cloned()`'s item is the item its call's row places, so `clone_value`'s
own read places it, and copies.rs calls it.

docs/readability.txt is blessed for one share: `renamed x$1`, 425.1 to
425.2 in 10,000 lines of the files both versions have. No rename was
added. `option_cloned_of_a_borrowed_refcell_names_std_line.js` is 8 lines
shorter, its row's key now fitting a line, with its two renames as they
were.

## Amendment: a function made a value is placed by how its callee calls it, a trait's method whose impl may track it too (2026-11-24, #2522)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named another
line than Rust's.

Three gaps of #2335's shim rule, each measured natively:
1. Generic code's `items.iter().map(T::clone).count()` of a `RefCell`
   named where `T::clone` is made a value. Rust names `FnMut::call_mut`'s
   shim, `function.rs:166:5`. `shim_location` asked only of a function
   that tracks its caller, and `Clone::clone`'s declaration doesn't, while
   a `RefCell`'s impl does. It now asks `value_tracks_caller`
   (std_types/location.rs), which is also true of a trait's method whose
   impl the instances may be one that tracks (#2457's
   `Located::gives_location`).
2. An `F: FnMut` a callee moves into `Option::map` is called through
   `FnOnce::call_once`, `function.rs:250:5`; its bound alone said
   `call_mut`. A call's argument is now placed by the callee's optimized
   MIR, its parameter followed by the same walk (`stored_through`),
   falling back to the bound only where that MIR says nothing:
   `Iterator::map` stores it in its `Map`. A value's own drop, a callee's
   unwind of what it moves on, is no use of it. A struct holding the
   value given whole to a call follows that field into the callee's MIR,
   and a closure capture is read by its body's `Fn*` call of it, which
   std's inlined `map_fold` is.
3. Compiled std (`--std-iterators`) gave `Cloned::fold`'s `T::clone`
   `std/lib.rs:1:1`, a dummy span, and `clone_try_fold`'s closure's clone
   no location, `<unknown>`. The first is the capture rule above. The
   second came from `gives_location`, which read "no caller seen" as "no
   instance": the closure is inlined into `Cloned::try_fold` and nothing
   names it. A body lowered with no seen caller now asks
   `tracking_impl_may_be`, as one with unseen callers does.

And generic code's dictionary clone takes the line the lowering raises at
(`Expr::raising_at`) where one is set, a `cloned()`'s row (#2509) or an
`Option<A>`'s clone line, as a concrete `RefCell`'s does. Two corpus
snapshots move, `callee_field_own_terms` and `cell_of_a_boxed_option`,
whose generic `Option<A>` clones now name std's `Option` clone row.

Held by the corpus cases `generic_clone_made_a_value_names_call_muts_line`,
`fn_mut_moved_into_option_map_names_call_onces_line`,
`clone_held_in_a_struct_given_whole_names_call_onces_line` and
`generic_cloned_of_a_held_refcell_panics`, by test/iterator-forms.test.ts'
`cloned_any_`, `cloned_count_` and `cloned_fold_` built of std's structs (no
longer `differs`, and test/known/metamorphic/2522.txt is gone), and by the
mutations `argument-mir-unread`, `value-drop-a-use`, `holder-moved-unread`,
`closure-capture-unread`, `trait-value-untracked`,
`model-location-not-raised` and `no-callers-no-location`. Still to do: a
tracked function returned as a value (#2530).

## Amendment: a tracked function a function returns is placed by its callers (2026-11-24, #2530)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named another
line than Rust's.

`fn copier() -> impl Fn(&RefCell<u8>) -> RefCell<u8> { RefCell::clone }`,
called `copier()(&x)` with `x` mutably borrowed, panics natively at
`Fn::call`'s shim, `function.rs:79:5`. rust-js named `copier`'s line, or,
since #2522, refused it in a binary as untold: "returned" was a use no rule
read. Now `stored_through` answers the return place `Returned`, and
`through` resolves it by the crate's callers of the function
(`returned_through`): each mention of it in the bodies the emitter reads
must be a direct call whose destination's uses say one way. A direct call
of a `Fn` trait's own method of the value, `<F as Fn>::call(&copy, ..)`,
says that trait, and of a reference `Fn::call`, as `&F`'s impls call it.
That rule was missing everywhere: a value put in a local and called that
way was untold. A function JS may call, or, of a library, another crate
may (`library::seen_outside`, #2015, #2403), keeps #2335's refusal, as its
callers can't all be seen. binaries.test.ts' untold list loses its returned
value, which its caller now tells. A callee's own return, in the MIR the walk
reads of std, is no return of the body lowered.

Held by `tracked_fn_returned_names_fn_calls_line`,
`generic_clone_returned_names_fn_calls_line`,
`tracked_fn_returned_by_a_pub_fn_refused` (compile-fail) and the mutations
`returned-place-unread`, `fn-trait-call-unread` and
`returned-unseen-callers-ignored`.


## Amendment: a `copied()` or a `Box` of a `Repeat` is a frame of its own (2026-11-27, #2507)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named
another line than Rust's.

Below a runtime adapter, `repeat(&x).copied().skip(1).count()` and
`Box::new(repeat(x)).skip(1).last()` named the adapter's line,
`skip.rs:78:19`, where Rust names the `copied()`'s, `copied.rs:93:17`, or
the box's, `boxed/iter.rs:57:17`: JS held each as what it holds, so no
frame of it stood between the adapter and the `Repeat` to give its line.

Where the code that makes it holds a `Repeat` in its type
(`holds_a_repeat`), each is now a frame in JS, which gives its own row as
it passes the step on, as each runtime adapter does: a `copied()` whose
`count` or `last` is its source's own is `$copied(it)`, never its source
(`copied_frames_a_repeat`, iterators/mir_steps/copies.rs, the one answer
of what's made and of where its step is called, which `repeat_frame_row`
is given), its items as they are, `$copied`'s copy now optional; a
box of it is its lend, `$lent(it, "boxed")`, as `box_hides_its_steps`
now says of it, whose `last` gives `boxLast`. The box hides the `Repeat`'s
own `count` too, which it never passes on, so a `Box<Repeat>`'s `count`
steps by `next` as std's default does, never `Repeat::count`'s panic, and
a `Box<dyn Iterator>` of one is its unsized lend. `repeat_frame_row` keeps
what JS still has no frame of: a box's `last` passed on to what it holds
(`passed_through_box`), and generic code's `copied()`, which is its
source. A box generic code makes of what it's given, or is given, is its
lend already, now at the box's line too.

Each is decided by type, so a chain that never asks `count` or `last`
of it holds the frame too, `$copied($repeating(1)).take(4)`: narrowing it
to where a consumer reaches it, with the frame kept where the value
escapes the body, is #2621.

What's left, `located-otherwise` and listed against #2507: generic code's
`copied()` of what its caller gives, below an adapter, which is its source
in JS, and a box stepped by its dictionary's steps (`impl-owned`'s), till
std's `Copied` is compiled (#1458). Held by the corpus cases
`repeat_copied_below_skip_names_copieds_line`,
`repeat_copied_below_enumerate_names_copieds_line`,
`repeat_boxed_below_skip_names_boxs_line` (renamed from `.._skips_line`
and `.._enumerates_line`, their `located-otherwise` gone),
`repeat_copied_of_a_box_below_skip_names_boxs_line`,
`repeat_boxed_twice_below_skip_names_boxs_line`,
`repeat_copied_in_a_chain_names_copieds_line`,
`repeat_boxed_in_generic_code_names_boxs_line`,
`repeat_boxed_given_to_generic_code_names_boxs_line`,
`repeat_copied_in_generic_code_names_copieds_line` and
`repeat_copied_in_generic_code_below_skip_names_skips_line`
(`located-otherwise`), each against native in both builds, the lowering
test "a frame of a Repeat's step the runtime has gives its line once", and
the mutations of each use.
