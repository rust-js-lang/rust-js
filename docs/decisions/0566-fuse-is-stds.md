# 0566. `Fuse` is std's; a specialization is picked by its caller's fact

Status: Accepted. Extends [0561](0561-std-bodies.md); supersedes
[0473](0473-fuse-is-rusts-fuse.md) and
[0549](0549-a-held-fuse-steps-what-a-use-site-gives.md), and
[0528](0528-fuse-and-enumerate-from-either-end.md)'s `Fuse`, whose
`Fuse` is std's own now; amends
[0543](0543-a-held-peekable-keeps-what-it-peeked-at.md), whose `IFused` a
compiled body asks, [0454](0454-a-mut-given-as-a-type-parameter.md),
whose `F: Fn(&mut T)` is given a function that takes the box apart,
[0552](0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md), whose
`IntoIter` held in a struct is held as it is and stepped, and
[0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what a `Fuse` steps,
what it keeps as it ends, what it drops and what it shows are std's, as its
bodies say, of a `FusedIterator` and of another, in generic code too.

## Context

ADR 0561 compiled `Peekable` and named `Fuse` next: `$fuse` (ADR 0473),
its back and its length (ADR 0528), a held one given a use site's by
`$knowing` (ADR 0549), its `Debug` and its clone written by hand, and
generic code's `fuse()` given `IFused`, whether its parameter is a
`FusedIterator` (ADR 0543). std's `Fuse<I>` is `{ iter: Option<I> }`, and
what its ten listed bodies are made of is safe but for one thing: `next`
and `next_back` go through `FuseImpl`, a trait only std can name, with two
impls of it, `impl<I: Iterator>`, which clears `iter` as it ends, and
`impl<I: FusedIterator>`, which keeps it, as its `Debug` shows and its
drop times. rustc picks between them as it monomorphizes; in a
polymorphic body it resolves the call to neither (`--std-coverage` says
`spec`). Compiling the bodies met five more shapes rust-js hadn't lowered:
`and_then_or_clear`, a function private to `Fuse`'s module, taking an
`f: impl FnOnce(&mut T)`; an inlined `?`, which leaves
`assume(discriminant(residual) == 0)` of a residual never written, and an
inlined `is_none()`, which compares a discriminant with a variant's; a
`Clone` whose only bound is `I: Clone`, with no `I: Iterator` to make the
struct's `next` of; and generic code holding `I::IntoIter` as a JS
iterator, `pair<I: IntoIterator>`, making a `Fuse` whose `iter` holds the
wrapper where a concrete site's `Clone` and `Debug` of `Fuse<Flip>` read
the struct, `[0]`: two shapes of one type, which `$iteratorWith` and
`$iteratorValue` had papered over.

## Decision

**`Fuse` is std's struct, compiled into `@rust-js/std` as `Peekable` is:
`Iterator::fuse`, its `next` and `size_hint`, `next_back` and `rfold`,
`len`, its clone, its `Debug`, and both `FuseImpl`s' `next` and
`next_back`, with `and_then_or_clear`. Which `FuseImpl` a call takes is
its caller's fact, `IFused`.** `$fuse` and every hand-modeled part of a
`Fuse` are gone.

- **A std-private trait's call whose impl rustc leaves to the caller's
  type is the impl its type parameter's being a `FusedIterator` picks.**
  `recognition::fused_specialization` is the one answer: of a call
  rustc resolves to no one impl, the method of the impl it selects for
  the body's bounds, the method of the impl that specializes it on
  `FusedIterator`, and the type whose being one picks. The compiled body
  takes the fact as any fact (ADR 0145), `fuseIterator_next(fuse,
  IIterator, IFused)`, and is `IFused ? fuseFuseImplIFusedIterator_next(..)
  : fuseFuseImplI_next(..)`; a concrete caller, which knows its type,
  takes one. The facts analysis asks `IFused` where a body read from its
  MIR makes such a call, and of a provided method's `Self`,
  `iterator_fuse(self, SelfIterator, SelfFused)`; a use site's impl
  method asks it of the impl method it resolves to, `Fuse<I>`'s
  `next_back`'s. `TypeFact::Fused` stays what ADR 0543 made it, the one
  answer to whether a parameter is a `FusedIterator`; nothing else picks.
  A specialization on a trait that isn't a fact, `TrustedLen`'s, is std's
  optimization of the impl selected, which changes nothing a program
  sees, as ADR 0561 says. An impl that specializes another is named with
  the traits its bounds add, `fuseFuseImplIFusedIterator`.
- **A function std keeps private that a compiled body calls is compiled
  with it**, as its closures are, `and_then_or_clear`: no program can
  name it, so no recognizer rewrites it.
- **A compiled std iterator's struct knows its `next` and its
  `size_hint` where it's made**, `$stdIterator(struct, { next, hint })`,
  the compiled impl's `size_hint` or the rewrite of one whose body reaches
  memory, `$peekableHint`; its clone steps as what it clones does,
  `$stdIterator({ iter: .. }, fuse.$steps)`, since `Clone` has no `I:
  Iterator`. So `size_hint()` of any compiled std iterator is
  `$sizeHint(it)`, its struct's own, and `len()` the compiled `len` or
  `$exactLen($sizeHint(it))`; `Iterator`'s methods of it never resolve to
  the library, as ADR 0561 says, but a provided method compiled,
  `fuse().peekable()`'s, does. A use site that steps it from its back or
  counts it by a JS helper, `rev()`'s, gives it its compiled `next_back`
  and `len` with that use's dictionaries, `$knowing(f, (it) =>
  fuseDoubleEndedIterator_next_back(it, IDoubleEndedIterator, IFused),
  ..)`, ADR 0549's form, of the struct itself; a direct `next_back()`,
  `len()` or `rfold()` is the library's call.
- **What std's inlining leaves is read as rust-js reads a dead switch**
  (ADR 0561): an `assume`, which runs nothing, goes, and a discriminant
  compared with a variant's is that variant's test, which nothing reads
  where the `assume` went.
- **A function given for an `F: FnOnce(&mut T)` of a `T` that's an object
  here takes generic code's box apart**, `(x0) => f(x0.value)`, one
  parameter at a time, `mut_refs::unboxed_by_function`: ADR 0454's second
  refusal, of a closure's parameter, is lifted; its others stand.
- **An `IntoIterator` whose `IntoIter` the signature holds in a type of
  the crate's or a compiled std type, `Pair<I::IntoIter>` given back, is
  given as it is** with its `IntoIterator` dictionary, as one bounded by
  `Clone` is (ADR 0552) and a parameter's own value held is (ADR 0411);
  not one in a std adapter, `Zip<I::IntoIter, J>`, a JS iterator of it.
  std's `IntoIterator` dictionary of a type known here carries the
  `Iterator` of what it makes, `IntoIterIterator`, as the trait declares
  its `IntoIter` and a crate impl's carries it (ADR 0527), and a value of
  `I::IntoIter` held as it is is stepped through it: `iterator_fuse(it,
  IIntoIterator.IntoIterIterator(), IIntoIterFused)`.
- **A library function's manifest steps a parameter by a supertrait of
  its bound too**: `rfold` of a held `Fuse<I: DoubleEndedIterator>` steps
  `I` by `Iterator`.

```js
// `Countdown(5).fuse()`, `Pair { i: i.into_iter().fuse() }`, `f.rev()`
iterator_fuse([5], { next: countdownIterator_next }, true)
function pair(i, IIntoIterator, IIntoIterFused) {
  return { i: iterator_fuse(IIntoIterator.into_iter(i), IIntoIterator.IntoIterIterator(), IIntoIterFused) };
}
$rev($knowing(f, (iterator) => fuseDoubleEndedIterator_next_back(iterator, { next_back: $nextBack }, true)))
```

## Why

- **It's std's**: `test/corpus/std_fuse.rs` drains a `FusedIterator`'s
  `Fuse`, another's and a range's and shows each, `Some` kept or `None`,
  steps one from its back, counts, clones, reverses and folds it, in
  generic code and concrete, against native Rust; the eleven programs
  that fused, `fused_iterators`, `held_fuse_back_drops`, `held_peekable`
  and the rest, print what they printed.
- **One mechanism**: the `FuseImpl` choice is the fact ADR 0543 already
  passed, read where std's body reads it; nothing of `Fuse` is written by
  hand, and `Peekable`'s latent two shapes, a source held as a wrapper
  by generic code and read as a struct by concrete code, are closed with
  it.
- **The gaps it found are closed in rust-js**, as a port's are:
  `function_given_for_a_mut_param.rs` gives a closure and a function for
  an `F: FnMut(&mut T)` of an object and of a number.

## Consequences

- **The app's JS**: a `fuse()` is `iterator_fuse(it, { next: $next },
  true)`, in a chain consumed in place too, which `consumed_fuses` had
  left as its source, the accepted cost, as a `peekable()` was; a `Fuse`'s
  `next_back()`, `len()` and `rfold()` are the library's calls with this
  use's dictionaries, and an impl method stepping a held `Fuse<I>` from
  its back takes `IFused`; its `rev()` is `$rev($knowing(..))`; a
  `Peekable`'s `size_hint()` is `$sizeHint(p)`; a generic function
  holding `I::IntoIter` in a struct takes `IIntoIterator`, `both`'s of
  `generic_into_iter_values`; std's `IntoIterator` dictionaries carry
  `IntoIterIterator`. Sixteen snapshots changed, each read.
- **The library's JS** shows what ADR 0561's next step, tidy of a std
  body's JS, is for: `{ TAG: "Continue", _0: v }._0` of an inlined
  `Try::branch`, `option = $some(x); const v = $someValue(option)`, and a
  `let x; let option;` hoisted above the branch that returns.
- **`size_hint()` of the crate's iterator with its own `size_hint`, given
  as it is to generic code, is a lazy iterator's**, `(0, None)`:
  `$sizeHint` of its struct, as before (ADR 0170); its `len()` through
  `ExactSizeIterator`'s dictionary is exact. A gap of its own.
- `docs/std-coverage.txt`: `Fuse` 10 of 14, `Iterator::fuse` known.
- **The readability gate** (ADR 0565): three shares grew, each by a
  tenth of a point in ten thousand lines or so, and are blessed here.
  `$iter`, 61.7 to 62.2: `fused_iterator`'s `sizes.iter().fuse().map(..)`
  and `chars().fuse().rev()` keep their `$iter`, now that a `fuse()` is
  std's call and no longer rewritten away (two), `both`'s `IntoIterator`
  dictionaries of `Vec`s (two), and `std_fuse`'s own (one). `return;`,
  9.5 to 9.6, and `renamed x$1`, 953 to 955.2: the new programs,
  `std_fuse` and `function_given_for_a_mut_param`, whose `main`s hold a
  `value$n` for each of many `{:?}` arguments; and the denominator
  shrank, `held_peekable` losing 410 lines of inlined `Fuse` `Debug` for
  `fuseDebug(..)`, which no counted shape credits.
- Of ADR 0454's refusals, a closure's parameter is lifted: an `Option<&mut
  T>`, a field and a `Vec<&mut T>` stand.
- **Next**: each adapter after `Fuse`, `Copied`'s derive among them, which
  a `Peekable<Copied<I>>` shows; and tidy of a std body's JS.

## What holds it

- `test/corpus/std_fuse.rs` and `function_given_for_a_mut_param.rs`;
  `fused_iterator`, `fused_iterators`, `fuse_size_hint`,
  `fuse_enumerate_ends`, `held_fuse_back`, `held_fuse_back_drops`,
  `held_peekable`, `adapter_drops`, `associated_iterator_steps`,
  `map_stepped_from_its_back` and `generic_into_iter_values` as they were,
  their JS reblessed.
- `test/std-package.test.ts`, the committed `@rust-js/std` with `Fuse`;
  `test/std-coverage.test.ts`; `test/runtime-package.test.ts` without
  `$fuse`; the architecture test, `$steps` named once, and the
  dispatchers' lengths.
- Mutations `fused-specialization-unseen`, `fused-impl-not-picked`,
  `fused-specialization-not-crates`, `fused-specialization-not-stepped`,
  `fused-specialization-fact-not-asked`, `private-callee-not-compiled`,
  `clone-steps-afresh`, `std-iterator-hint-missing`,
  `compiled-iterator-size-hint-refused`, `compiled-iterator-not-knowing`,
  `compiled-provided-of-compiled-iterator-not-crates`, `assume-kept`,
  `discriminant-compared-as-a-value`, `function-given-boxed`,
  `function-param-mut-refused`, `into-iter-held-in-struct-a-js-iterator`,
  `std-into-iterator-steps-missing`, `library-supertrait-not-stepped`,
  `std-callee-not-followed`, `std-provided-self-fact-not-asked`,
  `impl-method-facts-not-asked` and `library-fact-of-alias-not-asked`.
  `$fuse`'s own, and the hand-modeled `Fuse`'s, go with them.
