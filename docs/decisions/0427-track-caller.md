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

