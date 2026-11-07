// An iterator an adapter holds, closed when the adapter ends early, is
// dropped, or is done, however it's stepped: `drop`, its own drop, given what it hasn't given,
// once, as Rust drops it with the adapter (ADRs 0422, 0465, 0570); its back
// still its own, where it has one (ADR 0514). It's a `$Steps` (ADR 0669):
// its fold is its iterator's, after which it's done with; the crate's own
// fold takes it and drops it itself (#162), as std's compiled fold does,
// which leaves it taken, so its own `return` drops nothing again (ADR
// 0912): its lifecycle, `$state`, `$close`'s and `$takenBy`'s (#870).
function $closing(it, drop, slices = true) {
  const close = () => $close(closing, () => drop?.(it));
  // Stepped to its end, it's done with, as a chain consumed to its end
  // drops what it holds where the statement or the loop ends (ADR 0570).
  // Done, from its front or by its `nth` or `nth_back`, it's done with,
  // each way a consumer of JS's own steps it (#225).
  // Held, its owner drops it, where Rust does (#206).
  const done = (step) => {
    if (step.done && !closing.held) close();
    return step;
  };
  const closing = new $Steps(() => done(it.next()), close);
  closing.from = [it];
  // Of an array's items it owns, a `Vec`'s `IntoIter`'s, std's `nth_back`:
  // what it passes over dropped as one slice, front to back, as its
  // `advance_back_by` drops them, then its next from the back; not of one
  // that drops each as it's passed, `slices` false, a `Drain`'s (#234).
  // And from its front, its `advance_by` and `nth`, what they pass over
  // dropped as one slice too, by itself, as a `chain`'s `nth` asks it
  // (#250); how many there weren't. Of one given its own, `$slicing`'s,
  // which generic code's caller gives it, those (#703, #704).
  // A `Vec`'s `IntoIter`, whose `last` is its own (#648).
  const own = Object.hasOwn(it, "advanceBackBy");
  const intoIter = own || (slices && it.items !== undefined && it.nthBack === undefined && drop !== undefined);
  const slice = own || (intoIter && drop.each !== undefined);
  const advanceBackBy = own ? (m) => it.advanceBackBy(m, drop?.each) : (m) => $advanceSlice(it, m, drop.each, true);
  const advanceBy = own ? (m) => it.advanceBy(m, drop?.each) : (m) => $advanceSlice(it, m, drop.each);
  const sliceNth = (k) => done(advanceBy(k) > 0 ? { done: true, value: undefined } : it.next());
  const sliceNthBack = (k) => done(advanceBackBy(k) > 0 ? { done: true, value: undefined } : it.back());
  if (slice) {
    $owns(closing, "nthBack", "nth");
    // Each item's own drop, which what it gives on is dropped by (#261).
    closing.dropItem = drop?.each ?? it.dropItem;
  }
  closing.back = () => it.back();
  // Its length, its iterator's, where it knows it: what an `enumerate`
  // stepped from its back counts by (#211).
  if (it.len !== undefined) closing.len = () => it.len();
  // Its hint, what it steps', as dropping it adds no step (#733).
  closing.sizeHint = () => $sizeHint(it);
  // Its `nth`, its iterator's, the crate's own where it writes one, or a
  // compiled std struct's, which drops what it passes itself, `ownNth`, as
  // a `step_by` over it asks it (#261), given the items' drop, which std's
  // default `nth` of the crate's iterator drops what it passes by (#494).
  $ownsAs(closing, it, "nth");
  // And its `nth_back`, likewise, which a `rev()` over it says is its own
  // `nth` (#649).
  $ownsAs(closing, it, "nthBack");
  // Closed though `f` throws, a panic, as Rust's `fold` drops what's left
  // of what it took as it unwinds (#422); taken, not closed, where its
  // fold is its own, `own`, which took it and drops it as it unwinds (#423).
  // What drops with `f`, `ends`, first, as `fold(self, init, f)` drops `f`
  // before what's left of `self` (#710).
  const unwound = (fold, own, ends) => $unwinds(own ? () => $takenBy(closing, fold) : fold, ends, close);
  // Given `unwind`, what drops an accumulator a stage's panic strands, it
  // passes it on, so a step of the crate's own iterator that panics drops
  // a `last`'s kept item before what's left (#423, #426); and `ends`, what
  // drops with `f`, which its iterator's fold drops before what it has
  // left, or it does before it's closed (#710).
  const fold = (init, f, unwind, ends) => {
    const folded = unwound(
      () =>
        it.fold !== undefined ? it.fold(init, f, unwind, ends) : $foldDropping(it, init, (a, x) => f(a, x), unwind, ends),
      it.ownFold,
      ends,
    );
    $closedAfter(ends, close);
    return folded;
  };
  // Its `count`, the crate's own, likewise (#505).
  $ownsAs(closing, it, "count");
  const count = () => $takenBy(closing, () => it.count());
  // Its `last`, the crate's own, which takes it and drops what it has left
  // itself, as a `skip`'s `last` asks it (#494). Of an array's items it
  // owns, std's `IntoIter::last`, `next_back()`: its last item, then what
  // it has left dropped, front to back, as it's dropped; where one of
  // those drops panics, the item it took is leaked, as rustc doesn't drop
  // what a function gives back as a later drop unwinds (#648).
  if (it.ownLast || intoIter) $owns(closing, "last");
  const last = () => {
    if (intoIter) {
      const step = it.back();
      close();
      return step;
    }
    return $takenBy(closing, () => it.last());
  };
  // Its `nth_back` and `rfold`, its iterator's, the crate's own where it
  // writes them, done with after its `rfold` as after its `fold` (#220).
  const rfold = (init, f, unwind, ends) => {
    const folded = unwound(() => it.rfold(init, f, unwind, ends), it.ownRfold, ends);
    $closedAfter(ends, close);
    return folded;
  };
  // Its `try_fold`, its iterator's, done with where it isn't stopped.
  const tryFold = (init, f, broke, unwind, ends) => {
    const folded = $lent(it).tryFold(init, f, broke, unwind, ends);
    if (!broke(folded) && !closing.held) close();
    return folded;
  };
  $forward(closing, it, {
    advanceBackBy: slice ? advanceBackBy : undefined,
    // Its `advance_by`, its iterator's own, a compiled std struct's (#286).
    advanceBy: slice ? advanceBy : Object.hasOwn(it, "advanceBy") ? "advanceBy" : undefined,
    // Of one given its own, by its slice's with this one's drop.
    nth: own ? sliceNth : it.nth !== undefined ? (k, dropItem) => done(it.nth(k, dropItem)) : slice ? sliceNth : undefined,
    nthBack: own ? sliceNthBack : it.nthBack !== undefined ? (k) => done(it.nthBack(k)) : slice ? sliceNthBack : undefined,
    fold,
    count: it.ownCount ? count : undefined,
    last: it.ownLast || intoIter ? last : undefined,
    rfold: it.rfold !== undefined ? rfold : undefined,
    tryRfold: "tryRfold",
    tryFold,
  });
  return closing;
}

// An adapter that holds a function with a destructor, `map(f)`'s, closed to
// drop it, `drop`, once: what it steps closed, then its function dropped,
// as Rust drops an adapter's fields (ADRs 0485, 0991). Folded by value, its
// function dropped where std's fold drops it, `moved`, each as std's
// adapter writes its fold (#710): of a `map`'s, moved into the fold it asks
// of what it steps, after what drops with the `f` it's given, `"after"`, as
// `map_fold(f, g)` drops `g` first; of a `filter`'s, a `filter_map`'s and an
// `inspect`'s, before it, `"before"`; of a `flat_map`'s, alone, its `f`'s
// dropped after its fold, `"alone"`, as `FlattenCompat::fold` folds its
// `Map` by `&mut`; else kept, a `take_while`'s, a `map_while`'s or a
// `skip_while`'s, whose fold is its `try_fold`, dropped with it after what it
// steps.
function $holding(it, drop, moved) {
  const own = $onceOf(drop);
  // What it steps, then its function, though the first's drop throws, a
  // panic, as `Map`'s drop glue drops `iter`, then `f` (#762).
  const holding = $closing(it, (it) => $inTurn(() => it.return(), own));
  if (moved === undefined) return holding;
  // Its function and what drops with `f`, `ends`, in the order its fold
  // drops them, given to what it steps to drop as its fold ends; dropped
  // here as it's closed where that didn't, once.
  // Held, as `$consume` holds what it folds, so what it steps closes as
  // it's closed, after its function, not as it's stepped to its end, a
  // `step_by`'s source's (#734).
  const moving = (by) => (init, f, unwind, ends) => {
    holding.hold?.();
    const outer = $onceOf(ends);
    if (moved === "alone") return by(init, f, unwind, outer, own);
    const both = moved === "before" ? () => $closedAfter(own, outer) : () => $closedAfter(outer, own);
    return by(init, f, unwind, $onceOf(both));
  };
  const fold = holding.fold;
  holding.fold = moving((init, f, unwind, ends, alone) => {
    if (alone === undefined || it.fold === undefined) return fold(init, f, unwind, ends);
    // A `flat_map`'s: its fold drops `ends` after its source's, which it
    // gives its function to drop.
    const close = () => holding.return();
    const folded = $unwinds(() => it.fold(init, f, unwind, ends, alone), ends, close);
    $closedAfter(ends, close);
    return folded;
  });
  // Its `rfold`, its own, or of what knows its back but no `rfold` of
  // its own, an array's `map`, `$foldBack` of its back: moved as its fold
  // is, as std's `Map::rfold` moves its function into its source's (#734).
  const rfold =
    holding.rfold ??
    (holding.back === undefined
      ? undefined
      : (init, f, unwind, ends) =>
          $foldBack({ back: () => holding.back(), return: () => holding.return() }, init, f, unwind, ends));
  if (rfold !== undefined) holding.rfold = moving((init, f, unwind, ends) => rfold(init, f, unwind, ends));
  return holding;
}

// `it`, the array's items a `Vec`'s `IntoIter` owns, advanced by `m` from
// its front, or from its back, `back`: what it passes dropped as one slice,
// front to back, by `each`, as std's `advance_by` and `advance_back_by` drop
// them, the rest of it after one throws, a destructor's panic, as Rust's
// drop of a slice goes on; how many there weren't (#234, #250, #286, #670).
// Of items with nothing to drop, `each` none, none dropped.
function $advanceSlice(it, m, each, back = false) {
  const n = Math.min(m, it.len());
  const passed = back ? it.items.slice(it.end - n, it.end) : it.items.slice(it.at, it.at + n);
  if (back) it.end -= n;
  else it.at += n;
  if (each !== undefined) $dropItems(passed, each);
  return m - n;
}

// `it`, a `$iter` of the items a `Vec`'s, an array's or a `VecDeque`'s
// `IntoIter` owns, given its own `advance_by` and `advance_back_by`,
// std's for it: what they pass over dropped as one slice, front to back,
// by the `drop` each is given, or the items' own, `drop`, the rest of the
// slice after one throws; and its `nth` and `nth_back`, std's defaults of
// them, so its own, which drop what they pass themselves (`ownNth`), as
// an adapter over it asks; and its `last`, std's `IntoIter::last`,
// `next_back()`, what it has left then dropped as one slice, by the drop
// it's given, `$lastDropping`'s, as no owner drops a flatten's inner
// iterator after, or the items' own; given none, by its owner after,
// generic code or std's compiled adapter: where one of those drops
// panics, the item it took is leaked, as rustc doesn't drop what a
// function gives back as a later drop unwinds (#648). Its caller, which
// knows its type, gives it so where it's given to what doesn't: generic
// code, std's compiled adapters, a step whose helper sees only the JS
// iterator. A `Drain`'s, a
// B-tree's and a `BinaryHeap`'s aren't: theirs are std's defaults, each
// dropped as it's passed, a panic stopping them (#703, #704).
function $slicing(it, drop) {
  // Of one lent, `by_ref()`'s, what it lends, given them, and they passed
  // on, as std's `&mut I` passes on `I`'s `advance_by`, `nth`,
  // `advance_back_by` and `nth_back`; not its `last`, which takes it (#745).
  if (it.lends !== undefined) {
    if (it.lends.items === undefined) return it;
    $slicing(it.lends, drop);
    $forward(it, it.lends, { advanceBy: "advanceBy", advanceBackBy: "advanceBackBy", nth: "nth", nthBack: "nthBack" });
    it.dropItem = drop;
    return it;
  }
  // Given them already, or a JS iterator of its own, one closed, whose are.
  if (it.items === undefined || Object.hasOwn(it, "advanceBackBy")) return it;
  $forward(it, it, {
    advanceBy: (m, each = drop) => $advanceSlice(it, m, each),
    advanceBackBy: (m, each = drop) => $advanceSlice(it, m, each, true),
    nth: (k, each = drop) => (it.advanceBy(k, each) > 0 ? { done: true, value: undefined } : it.next()),
    nthBack: (k, each = drop) => (it.advanceBackBy(k, each) > 0 ? { done: true, value: undefined } : it.back()),
    last: (each = drop) => {
      const step = it.back();
      if (each !== undefined) it.advanceBy(it.len(), each);
      return step;
    },
  });
  $owns(it, "nth", "nthBack", "last");
  it.dropItem = drop;
  return it;
}

// The lifecycle of an iterator that drops what it holds once, `$closing`'s
// and std's compiled struct's, its `$state` (ADR 1092 S7, #870): open,
// none or "open", stepped; "taking", its own consumer running, the crate's
// `fold`, `rfold`, `count` or `last` or a compiled body that takes it,
// which drops it itself, as it unwinds too; "taken", that one done;
// "closed", dropped. Changed by these transitions alone. A move no Rust
// program makes, a drop of what its own consumer holds or a second
// consumer of what one took, is the runtime's bug, thrown as one.
//
// `it` closed, `drop()` once: from open, as `return()` drops it; nothing
// again where it's closed, or taken, whose consumer dropped it.
function $close(it, drop) {
  const state = it.$state ?? "open";
  if (state === "taking") throw $lifecycleBug(state, "closed");
  if (state !== "open") return;
  it.$state = "closed";
  drop();
}

// `run()`, `it`'s own consumer, which takes it and drops it itself: taken
// once it ends, though it throws, a panic, as it drops what it took as it
// unwinds, so a close after drops nothing again (#423, rd764). While it
// runs, it's taking, which nothing else may close: the crate's step that
// `$closing` asks of what it holds drops that itself. But a consumer that
// is `it`'s own code, `closesItself`, std's compiled body, drops it by its
// own `return()`, once, as Rust's drops `self`, `Take`'s `fold` (ADR 0912).
function $takenBy(it, run, closesItself = false) {
  const state = it.$state ?? "open";
  if (state !== "open") throw $lifecycleBug(state, "taking");
  if (!closesItself) it.$state = "taking";
  return $endsAfter(
    () => {
      it.$state = "taken";
    },
    () => run(),
  );
}

// `target` given `value`'s lifecycle, as a move of std's compiled struct,
// or a write of the whole of it, carries where it is (#537).
function $stateFrom(target, value) {
  target.$state = value.$state;
  return target;
}

// The runtime's bug: a lifecycle moved from `from` to `to`, which no Rust
// program does.
function $lifecycleBug(from, to) {
  return new $InternalError(`$closing moved from ${from} to ${to}`);
}
