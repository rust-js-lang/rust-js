# 0106. A trait's type parameters, associated types, generic methods and constants

Status: Accepted: a trait's type parameters, associated types, generic
methods and constants. Generic associated types, and a generic impl's
constant of its parameters, are to come. (Amended: a generic impl's
constant of its parameters is in, ADR 0176.) Extends [0049](0049-traits-and-generics.md)
and [0051](0051-generic-options.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

ADR 0049 supports the crate's own traits without type parameters of their
own. rustc's tests stop at the rest more than at anything else about traits:

```
119  generic trait parameters     trait Convert<T> { fn convert(&self) -> T; }
109  associated types             trait Source { type Item; }
 30  associated constants         trait Shape { const SIDES: u32; }
 20  generic trait methods        trait Shape { fn f<T>(&self, t: T); }
```

and one kind of test often has all three. Most of what a generic trait needs
was there already, for std's: `impl From<f64> for Meters` is a dictionary
of its own, `metersFromF64`, and a function's evidence is named apart.

## Decision

**A trait's type parameters are its dictionary's, as std's are.**

- **Each impl is a dictionary of its own**, named for the trait's arguments
  too: `metersConvertF64` and `metersConvertString` of two impls of
  `Convert<T>` for `Meters`. A function bounded by both is given both,
  `both(x, XConvertF64, XConvertString)`.
- **A `dyn` of one is a pair** (ADR 0049), its dictionary the impl's for its
  arguments: `&dyn Convert<String>`.
- **A supertrait's key is its trait's name**, `PartialEq`, or with the
  arguments the trait declares for it where it has two of one trait:
  `trait Both: Label<u32> + Label<String>` has `LabelU32` and `LabelString`.
  The key is the declaration's, not an impl's arguments', so a generic
  impl's dictionary and its caller agree: `Pair<A, B>: Label<A> + Label<B>`
  is `LabelA` and `LabelB` for `impl Pair<u32, String>` too. (Amended:
  where only a reference tells two apart, the reference is in the key too,
  `AddBase` and `AddRefBase` of `Add<Base> + for<'r> Add<&'r Base>`, as
  num-traits' `RefNum` has them, which was rust-js's internal error; the
  `supertraits_by_reference` corpus case compares it with native Rust.)
- **A higher-ranked supertrait, `for<'a> Greet<&'a str>`, is one
  dictionary**, its lifetime erased, as a higher-ranked bound is: rustc's
  trait selection takes none that's bound.
- **An `Option<T>` of a trait's `T` is ADR 0051's:** boxed where its payload
  could look like `None`, which `$some` and `$pop` box and nothing else
  does. So a concrete impl's `Option<u32>` and a generic impl's `Option<T>`
  are one representation, and generic code tells a taken `None` from none
  left: `count(&mut Stack { list: vec![None, None, Some(1)] })` is 3.

**An associated type is a type only a caller knows, as a type parameter
is:** `<S as Source>::Item` in generic code, and std's, `I::Item` of an
`Iterator`. Where rustc can say what it is, `<Count as Source>::Item` in
an impl's signature, it's that type.

- **What a type parameter's dictionaries are for, its are:** showing,
  comparing, cloning and defaulting one, and boxing an `Option` of it
  (ADR 0051). A bound on one is evidence as a type parameter's is,
  `shown(s, SSource, SItemDebug)` of `where S::Item: Debug`, given of the
  type rustc makes it for the caller, `String`'s `Debug`.
- **A bound the trait declares on one, `type Label: Display`, is in the
  trait's dictionary**, as a supertrait is, keyed by the type's name and the
  bound's, `LabelDisplay`: rustc proves `<L as Labeled>::Label: Display`
  from the trait, so generic code finds it in `L`'s `Labeled`.
- **A `dyn` of a trait with one says what it is**, `dyn Source<Item = u32>`.
- **A field of one is the type it stands for where its struct is used:**
  `Node<i32>`'s `value: K::Value` is an `Option<u32>`, as rustc's own
  types of places are. A field's type is normalized where it has one, so
  the struct's shape, what an `Option` of it is, and its clones and drops
  are the type's own; in generic code it's still a caller's.
  (Amended: a value of such a struct was an error, `<i32 as Key>::Value`,
  in 17 rustc tests.)
- **A value of one has nothing to drop only where nothing does**, the
  crate's own types and a library's: no drop function is given for one, as
  a type parameter's is (ADR 0098). Elsewhere it's an error.
- **One with lifetimes of its own**, `type Item<'a>`, is one too (ADR 0146).
  (Amended: it was an error.)

**A trait's generic method is given its own evidence where it's called,**
`describe<T: Display>`'s `TDisplay`:

- **Called where the impl is known, it's the impl's method**, given all of
  its evidence, as any generic function is.
- **Called through a dictionary, its own evidence comes after its
  arguments, in the order the trait declares it**: `SShape.describe(s, 7,
  { fmt: String })`. The dictionary's entry passes each on as the impl's
  bound it is, which may be in another order, `Square`'s `A: Debug +
  Clone`, with the impl's own evidence, made with the dictionary; and one
  that passes on just what it's given, in order, is the method itself.
- **A default copied into an impl takes it too**, after its arguments.
- **Where a type may have a destructor, it's an error:** called through a
  dictionary, it's given no drop function for its own type parameters, as
  an associated type isn't. (Amended: it's given them, as its trait
  declares them, ADR 0163.)

**A trait's constant is its value where the type is known, and its
dictionary's in generic code:**

- **Where rustc can say which impl's it is**, `Square::SIDES` or
  `<Square as Shape>::NAME`, it's the value rustc computed, written in
  place, as a type's own constant is (ADR 0031): `4`, or `"shape"` of a
  default.
- **In generic code, `S::SIDES` is `SShape.SIDES`**: each impl's dictionary
  has its value, the impl's or the trait's default, as rustc computed it
  for that impl, `{ SIDES: 4, NAME: "square", area: .. }`.
- **Of a type changed in place, it's a getter**, `get ZERO() { return { n:
  0 }; }`: each use is a value of its own, as a Rust constant's is, so a
  `bump` of one doesn't change the next. Of any other type, a value.
- **A dictionary has only the constants generic code reads**, or all, of a
  library, whose consumers may (ADR 0100): rustc evaluates only the
  constants a program uses, and a default no one reads may not evaluate,
  `360 / Self::SIDES` of a `Wrap` with none.
- **Not yet: a generic impl's constant of its parameters**, `impl<T: Foo>
  Foo for Proxy<T> { const X: i32 = T::X; }`, which rustc can't compute
  for every `T` at once. An error, as are a constant with parameters of
  its own, `const SIZE<T>`, and a `type const`.

## Why

- **A dictionary per impl is what rustc resolves:** `x.convert()` of a
  `Convert<String>` is that impl's method, and generic code is given the
  dictionary its bound names.
- **Naming a supertrait by its declaration** is the one name both sides of
  a generic call can compute: the impl knows its arguments, and generic
  code its own parameters.

## Consequences

- **A trait's type parameters are in** (`generic_traits`): two impls of one
  trait, a generic impl, a default method, a nullish payload through
  generic code, a `dyn`, and supertraits, of one trait twice, generic, and
  higher-ranked. Of the 119 rustc tests stopping at a trait's parameters,
  50 pass, none giving another answer; 26 stop at associated types and 15
  at generic methods. Found by them: two supertraits of one trait collided,
  and a higher-ranked one crashed rustc's trait selection.
- **Associated types are in** (`associated_types`): the crate's, a
  function's `Vec<S::Item>` for two impls, an equality bound, a `where`
  bound, a bound the trait declares, std's `I::Item`, and a `dyn` of one;
  and the diagnostics test's refusals, of a value with a destructor and a
  generic associated type. Of the 109 rustc tests stopping at associated
  types, 61 pass; of all 1,139 known failures, 129, none giving another
  answer. Found by them, as they got further: rustc's instance resolution
  panics on arguments it can't normalize, so they're normalized first, or
  not resolved; and an `extern` declaration of the crate's own
  `#[no_mangle]` function was a JS global no JS has, which is refused.
- **Generic methods are in** (`generic_trait_methods`): a default, one
  bounded by a closure, one with two bounds an impl orders the other way,
  called on a known type and from generic code, and the diagnostics test's
  refusal where a type has a destructor. Of the 20 rustc tests stopping at
  them, 11 pass; of all 1,139 known failures, 154, none giving another
  answer.
- **Associated constants are in** (`associated_consts`): known and generic
  uses, a default, a generic impl's constant that doesn't need its
  parameter, one of a type changed in place, and a default that would divide
  by zero, unread; and the diagnostics test's refusals. Of the 30 rustc tests
  stopping at them, 18 pass, none giving another answer. Found by them:
  evaluating every constant for a dictionary made rustc report a default's
  cycle and overflow that nothing used, and a `type const` crashed rustc's
  type checking.
- A trait's default method copied into a generic impl, `impl<T> .. for
  Stack<T>`, still stops at ADR 0098's destructors: `T` might have one.

## Amendment: an associated type's `Iterator` bound has its entry (#972)

An impl's dictionary gave the bounds its trait declares on an associated
type only of traits that have dictionaries, but generic code that gives
such a type to a function taking its iterator as it is (ADR 0411),
`advance(&mut items, None)` of a `C::Iter<'_>`, reads
`CContainer.IterIterator()`, as the route finds every declared bound: a
`TypeError` where native Rust runs. `Iterator`'s, `DoubleEndedIterator`'s
and `ExactSizeIterator`'s bounds have their entries too
(`has_item_bound_entry`), as std's `IntoIterator` dictionaries have
`IntoIterIterator` (ADR 0566), which the crate's own `IntoIterator` impls
now have as well.

Generic code has a value of such a type as a JS iterator (ADR 0061), so
each entry is that JS iterator's where the impl's dictionary made it one
(`made_js_iterator`, the one answer of what the impl's methods give, what
its entries step and what its drop drops, ADR 0178): `{ next: $next }`,
`{ next_back: $nextBack }`, `{ len }`. What it makes knows its back and its
length where the trait bounds the type so, as generic code steps it from
there; it threw `it.back is not a function`. It makes one of a range, of
the crate's own iterator, and now of the impl's own type parameter given
as it is, `type Iter = I`, by that one's dictionaries, `$given(it,
IIterator)`, which generic code stepped as a JS iterator where it was the
caller's own value; the impl is given those dictionaries, as the analysis
of what it steps reads the type's bounds (`iterator_steps`). Of a type
generic code has as std's own, a `Vec`'s `IntoIter`, the entry is its own.
Corpus case `associated_iterator_given_as_it_is`; the metamorphic net's
rewrites of `generic_associated_types` run as native.

## Amendment: what a caller's impls ask of what it passes through (2026-11-09, #1062)

Since #972 an impl whose associated type bound by `Iterator` is its own
parameter, `impl<T: Iterator> Parser for Token<T> { type Input = T }`, has
that bound's entries, `T`'s dictionaries: it steps `T`. A caller given its
own `I` and making that impl's dictionary only through another impl's
bounds, `Chain(Token(0), Token(1)).parse(i)`'s `Chain<Token<I>, _>: Parser`,
whose `L: Parser` is `Token<I>`'s, never gave `I`'s, as the analysis passed
on only what a call gives as it is: `I` stayed given as it is and its
dictionary was refused, "stepping `I`, which rust-js found nothing here
steps" (rustc's `bench/issue-32062.rs`). **The analysis follows each impl's
own bounds, as the lowering makes each impl's dictionary of what it's
given (`evidence_impl`), down to an impl given a parameter of the caller's
as it is: what that impl steps of it, the caller steps, and is given its
dictionaries.** Each bound of any lifetime, `F: for<'a> Fn(&'a T::Item)`,
is selected with its bound lifetimes erased, as rustc selects nothing that
names them. ADR 0262's case: none, the same values. Corpus case
`iterator_passed_through_a_chain_of_impls.rs`, an iterator only passed
through, one each impl steps, and an impl of such a bound; rustc's
`bench/issue-32062.rs` off the known failures; mutations
`impl-bounds-unwalked` and `impl-bounds-walked-with-bound-lifetimes`.

## Amendment: a dictionary's `clone` keeps its parameter's name to itself (#1246)

A `Clone` dictionary's `clone` is an arrow of one parameter, `value`, whose
body is made as the function's own statements are, its bindings named by
the function's fresh names. Of an `Option<Option<T>>` whose `Some` is
boxed (ADR 0051), that body binds what the outer `Some` holds, and where
`value` was still free in the function, as in the first of several such
dictionaries, it was named `value` too: `clone(value) { const value =
$someValue(value); .. }`, a module that doesn't parse. The arrow's
parameter is taken while its body is made, for its scope alone
(`clone_fn`): `const value$1 = $someValue(value)`. No other corpus JS
changed. ADR 0262's "results change" case (the program didn't load).

- `test/corpus/clone_dictionary_of_a_nested_option.rs`: four generic
  functions' clones of an `Option<Option<T>>` and a `Result<Option<T>, _>`,
  both builds against native Rust, through `METAMORPHIC=all`.
- The mutation is `clone-fn-parameter-free-to-its-body`.

## Amendment: std's struct of an adapter is made a JS iterator too (#1292)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results changed,
so Rust's are kept.

#972 made the value an impl gives for an associated type bound by
`Iterator` the JS iterator its dictionary makes of it, knowing its back
and its length where the trait bounds them so: a range, the crate's own
iterator, or the impl's type parameter given as it is. Std's struct of an
adapter wasn't made one. An impl's `type Iter = Map<Range<u32>, fn(u32) ->
u32>`, or a method's `-> impl DoubleEndedIterator`, gives std's compiled
struct (ADR 0545, amended, the user's decision (A)). Generic code took it
for a JS iterator that knows its back, as ADR 0480 says of an associated
type. So `(1..4).flat_map(S::items).rev()` gave `[]` where native Rust
gives `[20, 10, 0, 10, 0, 0]`, and `S::items(4).next_back()` threw.

`made_js_iterator` now answers yes of std's struct of an adapter too: the
entry makes it knowing its back by its compiled `next_back`
(`knowing_held`), asked of the method's output as the impl makes it, its
`<Tens as Source>::Iter` normalized, an `impl`'s hidden type revealed.
And `$iter` of what's neither an array nor an iterator that knows its
back throws a `TypeError`: a missing back is never stepped as an empty
array again.

`test/corpus/associated_iterator_of_stds_struct_from_its_back.rs` holds
`next_back`, `rev()` and `rfold` of both forms against native Rust in both
builds. `test/lowering-iterators.test.ts` holds the flatten, which the
build of std's structs alone refuses ("its constant size of `<S as
Source>::Iter`"), and `$iter`'s throw. Mutations
`made-js-iterator-of-std-struct-unseen`, `entry-output-unnormalized` and
`iter-of-what-knows-no-back-quiet`.
