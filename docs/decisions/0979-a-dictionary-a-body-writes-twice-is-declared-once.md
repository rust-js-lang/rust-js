# 0979. A dictionary a body writes twice is declared once at its top

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done"). The plan "rust-js: one representation per iterator", toward
its fourth phase, readability parity. Extends
[0098](0098-destructors.md)'s rule that a long drop is written once:
the same of a dictionary. Amended: a generic impl's dictionary, made of what
the function is given, too (#691).

Case ([0262](0262-when-rust-and-js-disagree.md)): none, no observable
difference. A dictionary of a type known whole is pure: objects and
functions of helpers and the program's items. Made once at a body's top,
it gives what each place made it gives.

## Context

With std's structs compiled, each compiled method is given its sources'
dictionaries, and each dictionary's entries give their sources' in turn.
The same literal was written again at every place it was asked: `rev()`
of a chain wrote `Chain`'s `DoubleEndedIterator`, four functions each
given four dictionaries, once for `$knowing` and once for `rev`'s bound,
and the same of each source. With std's structs alone `rev_chain` was
1,933 lines, 64 in the normal build, and the corpus was 22.3% longer
than the normal build's. The normal build repeated small ones too:
`{ len: (iterator) => $exactLen(countdownIterator_sizeHint(iterator)) }`
three times in a statement.

## Decision

A body lowered from its MIR, a function's or a closure's, keeps each
dictionary it makes that is of a type known whole and holds a function.
When the body is done, before tidy, each kept dictionary whose literal the
body writes twice or more is:

- declared once at the body's top, `const chainDoubleEndedIterator = {
  .. }`;
- named everywhere it was written, the largest first, so one inside
  another is named in its `const` too.

One written once stays where it's asked. A consumer that reads its
dictionary's literal, a function pointer that takes its `fmt` out of it,
still sees the literal: what's declared is decided only when the body is
done.

- **What it may read.** It reads only what a body's top can: the
  runtime's helpers, the program's items (`Known::Item`), and what its own
  functions bind. One that reads a variable of the body stays where it is.
- **Names.** Its type's and its trait's, `chainDoubleEndedIterator`,
  `u32Clone`. `unit`, `tuple` and `items` stand for types with no name of
  their own. The trait's alone is used where one literal serves several
  types: `clone`, the identity clone of many `Copy` types.
- `{ next: $next }`, of helpers alone, reads as well inline and stays.

## Consequences

`rev_chain` with std's structs is 526 lines. The corpus with std's
structs is 13.0% longer than the normal build, from 22.3%. 114 normal
snapshots change, by 968 lines added and 1,773 removed. The readability
ratchet's shares of `while (true)`, `Iterator.from` and renamed `x$1`
grow by a little: their counts are the same or nearly (two dictionaries
of one trait's name are `intoIterator` and `intoIterator$1`) in fewer
lines.

The mutations are `dictionary-declared-once-used`,
`dictionary-repeats-unnamed` and `dictionary-of-steps-declared`.

## Amendment: a generic impl's dictionary, made of what the function is given (#691)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, no observable
difference. A generic impl's dictionary is its function's call,
`holderTouch(TTouch)`, which `$traitImpl` caches by what it's given: each
call gives the same object, and making it runs nothing of the program's
but its entries' closures. Read once at the body's top, even ahead of a
branch not taken, it gives what each place it's asked gave.

A generic impl's dictionary was written at each place a body asked it:
a copied default calling its own impl through the dictionary's entry
(#690) wrote `holderTouch(TTouch).me(self)`, and so did generic code,
`both<T: Touch>(h: &mut Holder<T>)` calling `h.twice()` twice, and
`main` of a type known whole, `tExt({ next: $rangeNext })` twice. Nobody
writes a lookup again at every call.

- **What's kept.** Besides the literal of a type known whole, a body
  keeps each dictionary made that is a call of one of the program's items,
  a generic impl's function, of a type known whole or not, a copied
  default's of its own impl among them: its `Self` dictionary is the
  impl's, made of what the impl is given.
- **What it may read.** What a body's top can: the runtime's helpers, the
  program's items, what its own functions bind, and what the function is
  given (`given_names`, `traits/given.rs`): its dictionaries, `TTouch`, and
  what a copied default's made of them reads, its const parameters'
  values, facts, caller's location and drops, `dropI`. A closure's top
  reads its function's. A drop the body makes for itself isn't given, so
  a dictionary that reads one stays where it is.
- **Names.** A bound's dictionary's, as a function's parameter is named,
  of its type and its trait: `HolderTouch`, `RangePull`. Two of one name
  are told apart, `HolderTouch` and `HolderTouch$1`, as two literals are.
- **Where.** The body's top, as before, before tidy: a copied default's
  at its own, `const HolderTouch = holderTouch(TTouch);` ahead of
  `HolderTouch.me(self)`; one a closure writes twice at the closure's. One
  written once stays where it's asked. The largest is named first, as
  before, so a literal a call is given, `fuseDebug`'s `{ fmt }` of
  `peekableDebug(fuseDebug({ fmt }), debug)`, that's written nowhere else
  stays in the call's `const`, where it was a `const` of its own when the
  call was written three times.

Twenty-five corpus snapshots change, 162 lines added and 131 removed.
`test/corpus/generic_dictionary_declared_once.rs` runs against native
Rust: a copied default calling its impl's dictionary twice, and behind a
branch not taken; generic code, a closure in it and a branch not taken;
generic code given a drop its impl's dictionary takes, `slotRenew(TDefault,
dropT)`; `main`'s dictionaries of types known whole, one of another. The mutations
are `generic-dictionary-repeats-unnamed` (`traits/dictionaries.ts`),
`given-names-without-evidence` and `given-names-without-drops`
(`traits/given.ts`).

## Amendment: a dictionary that reads a JS global is declared once too (2026-11-10, #1239)

A dictionary a body writes twice is declared once at its top only where
what it reads is read the same there (`reads_only_at_top`): helpers, its
own parameters, what the body is given. A JS global, `Array` of
`Array.from`, is read the same anywhere, as no local is named it where its
module reads it (ADR 0352, `names::is_js_global`), so it counts as a
helper does. A std collection's `from_iter`, the collect generic code's own
is (ADR 0445, amended), reads `Array.from` and `new Set`, and was written
out at each use; `generic_collect`'s `vecFromIterator` is declared once
again. Held by the mutation `dictionary-reading-a-global-unkept`, whose
case is `dictionary_of_a_global_class_declared_once.rs`, an impl's
dictionary given the `Map` its own parameter's map class is, made twice,
`const BagTally = bagTally(colorHash, .., Map)`: generic_maps.rs held it
till #1363 made its `tCounting` one dictionary, given no class.

## Amendment: a runtime helper's of an impl's dictionary is named as that impl's (2026-11-11, #1156)

A dictionary a runtime helper makes of one impl's, `$readerDefaults(chainRead($sliceReader, $sliceReader))`,
std's defaults over a `Chain`'s `Read` (ADR 0425, amended), is a generic
impl's as `is_impl_call` asks it: the helper's one argument's. So a body
that writes it twice declares it once, `const ChainRead = ..`, as
`ChainBufRead` beside it is, not written out at each read. Applying this
rule to it was the coordinator's decision for the user. Held by the
mutation `helper-made-dictionary-unnamed` and
`test/corpus/io_read_chain_take_bytes.js`.

## Amendment: an entry of a given dictionary, called (2026-11-11, #1355)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, no observable
difference. A given dictionary's entry is a function of no arguments that
gives a dictionary, its supertrait's, `VOrd.PartialOrd()`, or of a bound
its trait declares of its associated type, `SSource.PairsIntoIterator()`
(ADR 0106): what each call gives does the same, as a generic impl's call
does, and making it runs nothing of the program's. Read once at the body's
top, even ahead of a branch not taken, it gives what each place it's
asked gave, of a dictionary always given. A `Copy` bound's is
`undefined` where each value is its own copy (ADR 0289), read only behind
the test of it, `TCopy ? TCopy.copy(x) : x`: an entry of it read at the
top, ahead of that test, would throw. So only an entry of a dictionary
its owner says is always given is declared (`dictionaries.rs`'
`always_given`, every bound's but a `Copy`'s); another stays where it's
written. The unit test
`a_dictionary_that_may_be_undefined_keeps_its_entry_inline` and the
mutation `entry-of-a-maybe-undefined-dictionary-declared` hold it.

Generic code wrote such a call at each place it read the dictionary, the
`IntoIterator`'s twice for each inner item a flatten makes,
`$given(SSource.PairsIntoIterator().intoIter(..),
SSource.PairsIntoIterator().IntoIterIterator(), ..)`. A body writing one
twice or more now declares it once at its top, as the kept are, named by
its entry, `const PairsIntoIterator = SSource.PairsIntoIterator();`
(`traits/dictionaries/declared.rs`' `given_entries`, read where the body is
done, `declare_kept`, as the route that makes it, `evidence_for`, has no
body to keep it in). One written once stays where it's asked. Six corpus
snapshots and `@rust-js/std`'s `iter.js` change: `const PartialOrd =
VOrd.PartialOrd();`, `const FromResidual = RTry.FromResidual();`. The
mutation is `given-entries-undeclared` (`lower/traits/dictionaries/declared.ts`).

## Amendment: a closure's own declaration of a dictionary its body declares is its body's (2026-11-24, #2494)

A closure lowered from its own MIR declares the dictionaries it writes
twice at its top, as any body does. When the body around it then writes
the same dictionary twice, counting the closure's declaration as one use,
the closure's `const` became a bare alias of the body's,
`const vecIndex = index;`. That turned the closure's arrow into a block,
and the body's name took a suffix, `vecDequeIndex$1`, because the closure
had claimed the plain one. Now `declare_repeated` removes each nested
`const` of the dictionary it declares, and that block's reads read the
body's name. Names are spelled when printed, so the body's is
`vecDequeIndex` again. No behavior changes: the case of ADR 0262 is
"none", a readability fix. Held by `index_through_a_generic_bound`'s
snapshot, with three more cases' moves (`back_predicate_unwinds`,
`flatten_folds_reach_its_sources_own`, `flatten_from_both_ends_drops_its_inners`),
and the mutation `nested-alias-kept`.

## Amendment (2026-11-24, #1301): std's readers' dictionaries are named one way

A `Cursor`'s dictionary is one object of every trait it's asked,
`$cursorReader(TAsRef)` for its `Read`, `BufRead` and `Seek` alike. Named by
its type's and the trait's, it was `cursorRead`; and where one literal served
several traits, by the trait's alone, `read` in one function and `seek` in
another, though it's the same object. It's named by what makes it,
`cursorReader`. A `BufReader`'s, a runtime factory's call per trait,
`$bufReader(RRead)` and `$bufReader(RSeek)`, was named as a bound's,
`BufReaderRead`, as the program's generic impls are; it's named as any other
type's, `bufReaderRead` and `bufReaderSeek`. The program's generic impls,
and a runtime helper wrapping one, `$readerDefaults(takeRead(..), ..)`, keep
the bound's form. Mutations `cursor-dictionary-named-by-trait`,
`buf-reader-dictionary-bound-style` and `reader-defaults-named-as-a-factory`.

A body with two cursors of two types, a `Cursor<&[u8]>` and a
`Cursor<String>` (io_cursor.rs), named them `read` and `cursorRead`, apart
only by the trait word one fell back to; they're `cursorReader` and
`cursorReader$1` now, so docs/readability.txt's renamed `x$1` share grows by
one name, 425 to 425.2 in 10,000 lines: an honest `$1` in place of a name
that said a `Read` of an object every trait reads.
