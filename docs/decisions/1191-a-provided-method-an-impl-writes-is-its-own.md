# 1191. A provided method an impl writes is its own, through generic code too

Status: Accepted (2026-11-24, the lead's decision for #2414, step 1, decided
for the user overnight). Amends [0185](0185-library-trait-defaults.md)
and [0561](0561-std-bodies.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Which impl's method a
call runs is observable, so the call runs the one Rust's dispatch picks.

## Context

A std trait's provided method compiled into `@rust-js/std` (ADR 0561),
called on a type parameter or a `dyn`, was that default function over
`Self`, `bufRead_readLine(r, buf, RBufRead, ..)`, whatever impl the value's
type had. That holds only where no impl writes its own. Rust's dispatch
picks the impl's own where it writes one, and many do:

- a crate may write any stable provided method: a reader's `read_line`, a
  `Seek`'s `rewind`, an iterator's `partition`. Generic code ran std's
  default: `read_line` read the buffer where the override ran, `rewind` left
  the position, `partition` gave std's sides (#2414);
- std's own impls write `Extend`'s unstable `extend_one`, `extend_reserve`
  and `extend_one_unchecked`: a pair's, which splits each item between its
  sides, a `Vec`'s `reserve`. std's compiled `unzip()` (#2395) reached them
  through `default_extend` and recursed, as a pair's default `extend_one`
  is its `extend`.

## Decision

**One owner says whether an impl in reach writes its own of a provided
method; where one does, a call on a type parameter or a `dyn` is Rust's
dispatch.**

- `recognition/overrides.rs` `overridden_in_reach(tcx, provided)`: whether
  any impl rustc loads, std's, the crate's or a library's, writes it,
  `impl_item_implementor_ids` of each of its trait's impls. A crate built on
  this one, which may write one too, is #2414's second step (its manifest).
- `in_std_dictionary` asks it, of the methods compiled into `@rust-js/std`,
  so each dictionary of the trait carries the method its impl writes: the
  crate's own (impls.rs's library-default path) and std's compiled impls'
  (`tuple2Extend`'s). A hand-made dictionary of a std collection carries
  what its std impl writes observably: `extendReserve`, its `reserve`
  (`traits/dictionaries/extend_entries.rs`), only where rustc says the
  impl writes `extend_reserve`, refused for a type with no form of it.
  std's `extend_one`, a collection's `push` or `insert`, is what the
  default, `extend(Some(item))`, does to it, so it carries none.
- traits/calls.rs asks it too. Where it says no, the call is the default
  over `Self` as before. Where it says yes, `$provided(dictionary, key,
  entry, () => default)` (`provided_call`): the dictionary's own where its
  impl writes one, given `entry`, what that entry takes
  (`through_dictionary`'s, or a `dyn`'s pair and its rest, `dyn_entry`'s);
  else the default's call, made only then, of what it takes
  (`compiled_default_args`, the one list of it), as what it converts may
  change what it's given: `Iterator.from` arms a `$Steps` to close (amended
  below). `key` is `bindings::fn_name`, the naming owner's name of the
  entry. The entry list is names, dictionaries and drops.
- `DICTIONARY_METHODS` stays the one list of what a byte source writes;
  `$readerDefaults` and `$seekDefaults` are untouched.

**What isn't here.** An `Iterator`'s provided method: generic code holds an
iterator as a JS iterator (ADR 0061), with no `Iterator` dictionary, and
in-place consumers, `$partition`, never reach traits/calls.rs. A crate
iterator's own steps ride on its value (`OwnMethods`), so #2414's
`partition`/`peekable` case is #2112 Change A step 3 phase 3's: `OwnMethods`
carries each provided method `overridden_in_reach` names.

## Why

- **Rust's rule, one owner.** Which impl runs is what rustc says of the
  impls it loads; nothing restates it per trait.
- **Output moves only where something writes its own.** Where nothing does,
  the call is the default over `Self`, as it was.
- **One helper of the dispatch alone.** `$provided` encodes "the impl's own,
  else the default", no std body: std's defaults stay compiled from std.

## Alternatives

- **Every dictionary of the trait completed with std's default.** The
  runtime's hand-made dictionaries can't import `@rust-js/std`'s compiled
  defaults, so it meant runtime copies of std's bodies or a third list.
- **Every stable provided method through the dictionary.** Sound, but every
  generic `it.peekable()` and `r.read_line(..)` would move in every program.

## Consequences

- std's forwarders write `read_line`, `read_until`, `skip_until` and
  `rewind` (`&mut B`, `Box`, `Empty`, `Chain`, `StdinLock`, `Arc`), so
  generic code's calls of them are `$provided` in every program.
- std's compiled `SpecExtend::extend` calls `$provided` of its collection's
  `extendReserve` and `extendOne`.
- Corpus, against native: `provided_methods_a_type_writes_are_its_own` (a
  crate's `read_line`, through a type parameter and a `dyn`),
  `rewind_a_type_writes_is_its_own`,
  `provided_io_methods_of_std_readers_by_generic_code`,
  `extend_reserve_of_sides_generic_code_fills` (capacities of a `Vec` and a
  `VecDeque` side, of a `TrustedLen` and a `filter`),
  `extend_reserve_of_an_endless_source_panics_in_order` (hashbrown's room
  check before a `Vec`'s, and after), and
  `hash_set_extend_reserves_half_once_it_holds_items` (wasm32: hashbrown's
  `div_ceil(2)` of a set that holds items). Its empty-set branch, the whole
  lower bound, can't be told from half against native, as wasm32 aborts on
  the panic and a 64-bit native has room for either: so the rule is pinned
  to hashbrown's source where it's mirrored, a `std-source-pin` line in
  `extend_entries.rs`, which test/std-specializations.test.ts checks as it
  checks `OBSERVABLY_DEFAULT`'s rows.
- Mutations: `provided-overridden-nowhere`, `overridden-provided-not-in-dictionary`,
  `provided-overridden-shortcut`, `provided-key-not-its-entry-name`,
  `provided-dyn-receiver-its-box`, `provided-always-the-default`,
  `extend-reserve-entry-never`, `hash-extend-reserve-whole`,
  `hash-extend-reserve-unchecked`.

## Amended: an iterator's own, on its value (2026-11-24, #2414, #2112 Change A step 3e)

Generic code holds an iterator as a JS iterator (ADR 0061), with no
`Iterator` dictionary, so an `Iterator` provided method a crate's iterator
writes itself, `partition`, `peekable`, has no dictionary to ride on. Its
value does: `$iterator(..)` carries the crate's own steps already
(`OwnMethods`), and now, as its last argument, each provided method
compiled into `@rust-js/std` that an impl in reach overrides and its impl
writes (`own_provided`), under `own<Name>` (`own_provided_key`, of the
naming owner's name, as `$ownStep` names `ownCount`, never JS's own `map`
or `filter`), each bound to its value as `self`; and its `Iterator`
dictionary carries each by its entry's name, `partition`, as it carries
its own `fold` beside the value's `ownFold`. A call of one on a type
parameter is Rust's dispatch, as `provided_dispatch` says (traits/calls.rs,
the one owner): `$provided` of the dictionary it's given, given `self` and
what follows; or, given none (`evidence_for` none), of the value, given
what follows `self`. In traits/calls.rs, and where the recognizers lower a
combinator, by `provided_on_value`, which spills what's given only there.
Its own consumes and drops `self`, so generic code drops the value after
only where it didn't run: `dropped_consumed` ands in `provided_took`,
`!it.ownPartition` or `!$took(IIterator.partition, iter)`. A `&mut I` and
a `Box<I>` pass none on, as std's don't, and their lends carry no `own*`
key; a `rev()` and an adapter pass none on.

Each entry, the value's and the dictionary's, is `provided_entry`'s
(traits/evidence.rs, beside `own_evidence`, the one owner): a function of
what the call gives, its values, then the trait method's own evidence
(`own_slots_through`, `own_evidence`'s list, its bounds normalized to the
type's terms, `B: Extend<u32>`), which gives the impl's function what it
takes after its values by `evidence_args_given`: an impl with type
parameters of its own its impl's evidence first, `(iterator, f, BDefault,
BExtendU32) => wIterator_partition(iterator, f, counterIterator, BDefault,
BExtendU32)` of a `W<Counter>`'s; an impl whose method asks fewer bounds
than the trait's, `B: Default` alone, only those; `counterIterator_partition`
itself where it's given just what the call gives. A const's or a fact's
slot, which none of `Iterator`'s methods has, is refused. `evidence_args_given` matches a given bound to the impl's however each is spelled, normalized: `B: Extend<Self::Item>`, either's, is `B: Extend<u32>`. A call through
a dictionary gives its entry its receiver as generic code holds it, the
impl's own value, never the form std's compiled body steps `self` in
(`through_dictionary`, #2423).

`$provided`'s default is its call in a thunk, `() => default`, made only
where it runs, one shape for every call, a dictionary's and a value's; the
io cases and std's calls moved to it.

Corpus `iterator_provided_methods_a_type_writes_are_its_own` (own
`partition` and `peekable` through a type parameter, given no dictionary
and given one, `impl Into2<I>`, each dropped once; std's through a `&mut`,
a `&mut I` generic code holds, a `Box` and a `filter`) and
`generic_iterator_writing_partition_given_to_generic_code` (a `W<L>`'s
own of a range, a `W<Counter>` and a `W<W<Counter>>`, each forwarding to
what it wraps; one given to generic code that never calls it; an impl's
`partition` asking `B: Default` alone) agree with native.
Mutations: `iterator-provided-unbound`, `own-provided-uncarried`,
`own-provided-undictionaried`, `provided-never-on-the-value`,
`provided-dictionary-taken-for-the-value`, `provided-on-a-lent-value`,
`provided-took-dropped-again`, `provided-default-made-first`,
`provided-entry-given-none-of-its-bounds`, `provided-entry-bounds-unnormalized`,
`provided-entry-never-eta-reduced`, `entry-receiver-crossed`,
`given-bound-matched-as-spelled`.

Left (#2421, the lead's form): a provided method std's lowering writes
itself (`position`, `any`, `find`, `max`..), which a crate's override
never reaches through generic code; and a value crossing the two holdings,
generic code given a dictionary handing it to generic code given none, or
back, which runs std's. Both by one holding, a `provided` object
`$iterator` and the dictionary carry and `$given` passes on, and one
dispatch on the steps generic code holds.

## Amended: one holding, one dispatch, every provided method (2026-11-24, #2421)

The amendment above covered the provided methods compiled into
`@rust-js/std`, dispatched at three places (traits/calls.rs, the
combinators' lowering, and the drop after a consumer, `provided_took`),
the value carrying its own and the dictionary its own apart. A crate's own
`position`, `any`, `find` or `max`, which rust-js lowers itself, ran std's
through generic code; and a value handed from generic code given its
dictionary to generic code given none, or back, lost its own.

- **One rule** (recognition/overrides.rs `dispatched_override`): a provided
  method of `Iterator` an impl not std's writes, and none of the steps a
  JS iterator carries of its own already (`OwnStep`'s, `size_hint`). std's
  own impls' are what their JS forms step as and give the default's
  result, so they ask none. `collect` is left as it was, undispatched,
  pending the user's decision of whether an impl's own `collect` is ADR
  0262's exception or its entry carries the facts it asks (#2430):
  itertools writes it for its generic adapters, whose entries ask a fact of
  `T::Item` a generic maker's dictionary doesn't carry.
- **One holding:** the impl's own of each, `provided: { ownPosition:
  counterIterator_position }`, the object `$iterator` takes, which the
  crate's iterator's dictionary carries too and `$given` passes on to the JS
  iterator it makes: so a value crossing between the two holdings keeps
  its own.
- **One dispatch,** where every call MIR lowers is asked
  (`dispatch/provided.rs`, `mir_call_values_dispatched`): where `Self` is
  a type parameter (`provided_dispatches`), `$provided(steps, "ownX",
  [..], () => { ..; return v; })`, `steps` the JS iterator generic code
  steps (`provided_steps`: the value given no dictionary, else `$given(it,
  IIterator)`; of a `&mut I` given as a value, a `by_ref()`'s, what it
  lends, `$referent`'s, as Rust's `I::position` runs on the `I`), and the default std's lowering of the call,
  `mir_call_values_of`'s, statements and all, unchanged, made only where it
  runs: what it drops after it too, so the impl's own, which drops what it
  consumes, is never followed by a second drop. traits/calls.rs's Iterator
  branch, the combinators' dispatch and `provided_took` are gone.
- Each entry is `provided_entry`'s; one the call gives drops after its
  values that the impl's function takes none of names none of them, so it's
  the function itself.

Output moves only where an impl not std's writes one of them: of the
corpus, `std_method_of_a_mut_to_a_held_mut`, whose iterator writes `find`.

Corpus `iterator_overrides_std_lowers_itself` (own `position` through a
type parameter, a `&mut I` and a `by_ref()`; `any`, `find`, `max`; a
`map` of it, std's; both crossings) agrees with native. Mutations:
`provided-never-dispatched`, `provided-default-made-before`,
`dispatched-override-of-its-own-steps`, `provided-on-a-lent-value`,
`provided-dictionary-taken-for-the-value`, `own-provided-undictionaried`
retargeted; `provided-lend-asked-as-it-is`, `referent-stops-at-the-lend`. `$referent`
walks a lend of a lend to its end, but no mutation of that can be caught on
this path: `$lent` makes a `&mut` of a `&mut` the lend itself, so a
`by_ref().by_ref()` is one lend. `provided-never-on-the-value` and `provided-took-dropped-again`
retired with their sites.

## Amended: the facts an override's entry takes, one calling convention (2026-11-24, #2430, staged)

An override's entry took the trait method's own evidence and refused a fact
slot, so an override whose own body asks a layout fact of what the call
knows, an itertools adapter's `collect` of `T::Item`, couldn't be made
where only a generic maker holds it. Now (traits/evidence.rs
`provided_fact_convention`) each provided method has one calling
convention for its overrides' entries: of its own fact slots, those some
override in reach, an impl not std's that writes it, asks of the same type,
in the method's slot order, decided once. Every entry takes exactly those
(`provided_entry`, matched to the impl's own facts by type through
`GivenEvidence`'s `facts`, normalized as bounds are), and the call passes
exactly those (`own_evidence_keeping`), or is refused there, naming the
override that asks the fact it can't name. A fact only std's default asks,
its `Vec`'s growth, isn't in the convention: std's runs in the thunk by its
own lowering, which gives it itself. No argument is ever an `undefined` no
one reads (the lead's rule).

`collect` stays excluded, and #2430 open: itertools' own `collect_vec`,
`fn collect_vec(self) -> Vec<Self::Item> { self.collect() }`, would pass
its overrides the `FromIterator` dictionary of `Vec<Self::Item>`, which
needs the item's layout, which generic code over `Self` doesn't carry; so
dispatching it would refuse itertools. That's #2442 (dictionaries and
generic code carrying associated types' facts), which blocks #2430; the
convention is what #2430 builds on once it's done. No output moves. Of the
methods dispatched today no override asks a fact the trait's own slots
name, so the convention is empty for each and no mutation of it is caught
by output yet; #2430's case will (a crate's own `collect` through generic
code, `target/` until then).

## Amended: an impl's own `collect` is dispatched (2026-11-24, #2430)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a crate's iterator
that writes its own `collect` ran std's where generic code collected it,
and printed what Rust's dispatch doesn't.

With #2442 the entry of a std collection's `FromIterator` asks its caller
what it collects by, so `collect` leaves `dispatched_override`'s
exclusion: generic code's `it.collect()` of a type parameter is
`$provided(it, "ownCollect", [BFromIterator], () => ..)`, std's collect in
the thunk, the override given the call's own `B: FromIterator`
dictionary. Whether a call dispatches is one answer,
`recognition::dispatched_call` (`Self` a type parameter and
`dispatched_override`), which the lowering's `provided_dispatches` and the
facts analysis's `collects_run` both ask: a dispatched `collect()` gives
its override's entry that dictionary, so what the dictionary's `fromIter`
collects by is asked of the caller, where std's own collect, lowered in
place, gives none. Without that, itertools' `collect_vec`, `fn
collect_vec(self) -> Vec<Self::Item> { self.collect() }`, was refused its
items' alignment, which the crate corpus caught.

No program's JS but the new case's moved: the corpus, the crate corpus
(itertools' rows too) and React's port are as they were, as none of them
writes its own `collect`. Held by `iterator_own_collect_through_generic_code.rs`
(a crate's own `collect` through generic code into a `Vec` and a
`BTreeSet`, std's through a `&mut` of it and a `map` of it) and the
mutations `entry-collects-of-a-dispatched-collect-unasked` and
`provided-on-a-lent-value`, which moved with the rule to
recognition/overrides.rs.

An override's own ask of a fact of the method's own type parameters, an
own `collect`'s `size_of::<B>()`, isn't in the convention, which holds
only the fact slots the method itself has, std's `collect` none of `B`'s.
Its entry can't be given it, so it's refused where it's made, naming the
override and the fact (`provided_entry`), never asked of what the entry
can't name: a refusal of valid Rust, never another program's output,
until generic code's dispatch gives an override those too (#2449, the
lead's decision). Held by
`iterator_own_collect_asking_its_outputs_size_refused.rs` and the mutation
`provided-entry-own-parameters-fact-unrefused`.
