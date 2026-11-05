// An iterator an adapter holds, closed when the adapter ends early, is
// dropped, or is done, however it's stepped: `drop`, its own drop, given what it hasn't given,
// once, as Rust drops it with the adapter (ADRs 0422, 0465, 0570); its back
// still its own, where it has one (ADR 0514). It's a `$Steps` (ADR 0669):
// its fold is its iterator's, after which it's done with; the crate's own
// fold takes it and drops it itself (#162), as std's compiled fold does,
// which marks it `$closed`, so its own `return` drops nothing again (ADR
// 0912).
function $closing(it, drop, slices = true) {
  let closed = false;
  const close = () => {
    if (!closed) {
      closed = true;
      drop?.(it);
    }
  };
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
    closing.ownNthBack = true;
    closing.ownNth = true;
    // Each item's own drop, which what it gives on is dropped by (#261).
    closing.dropItem = drop?.each ?? it.dropItem;
  }
  closing.back = () => it.back();
  // Its length, its iterator's, where it knows it: what an `enumerate`
  // stepped from its back counts by (#211).
  if (it.len !== undefined) closing.len = () => it.len();
  // Its `nth`, its iterator's, the crate's own where it writes one, or a
  // compiled std struct's, which drops what it passes itself, `ownNth`, as
  // a `step_by` over it asks it (#261), given the items' drop, which std's
  // default `nth` of the crate's iterator drops what it passes by (#494).
  if (it.ownNth) closing.ownNth = true;
  // And its `nth_back`, likewise, which a `rev()` over it says is its own
  // `nth` (#649).
  if (it.ownNthBack) closing.ownNthBack = true;
  // Closed though `f` throws, a panic, as Rust's `fold` drops what's left
  // of what it took as it unwinds (#422); done with, not closed, where its
  // fold is its own, `own`, which took it and drops it as it unwinds (#423).
  // What drops with `f`, `ends`, first, as `fold(self, init, f)` drops `f`
  // before what's left of `self` (#710).
  const unwound = (fold, own, ends) => {
    try {
      return fold();
    } catch (error) {
      if (own) closed = true;
      $closedAfter(ends, close);
      throw error;
    }
  };
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
    if (it.ownFold) closed = true;
    $closedAfter(ends, close);
    return folded;
  };
  // Its `count`, the crate's own, likewise (#505).
  if (it.ownCount) closing.ownCount = true;
  const count = () => {
    closed = true;
    return it.count();
  };
  // Its `last`, the crate's own, which takes it and drops what it has left
  // itself, as a `skip`'s `last` asks it (#494). Of an array's items it
  // owns, std's `IntoIter::last`, `next_back()`: its last item, then what
  // it has left dropped, front to back, as it's dropped; where one of
  // those drops panics, the item it took is leaked, as rustc doesn't drop
  // what a function gives back as a later drop unwinds (#648).
  if (it.ownLast || intoIter) closing.ownLast = true;
  const last = () => {
    if (intoIter) {
      const step = it.back();
      close();
      return step;
    }
    closed = true;
    return it.last();
  };
  // Its `nth_back` and `rfold`, its iterator's, the crate's own where it
  // writes them, done with after its `rfold` as after its `fold` (#220).
  const rfold = (init, f, unwind, ends) => {
    const folded = unwound(() => it.rfold(init, f, unwind, ends), it.ownRfold, ends);
    if (it.ownRfold) closed = true;
    $closedAfter(ends, close);
    return folded;
  };
  // Its `try_fold`, its iterator's, done with where it isn't stopped.
  const tryFold = (init, f, broke, unwind) => {
    const folded = $lent(it).tryFold(init, f, broke, unwind);
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
  let dropped = false;
  const own = () => {
    if (!dropped) {
      dropped = true;
      drop();
    }
  };
  const holding = $closing(it, (it) => {
    it.return();
    own();
  });
  if (moved === undefined) return holding;
  const once = (ends) => {
    let done = false;
    return () => {
      if (!done) {
        done = true;
        ends?.();
      }
    };
  };
  // Its function and what drops with `f`, `ends`, in the order its fold
  // drops them, given to what it steps to drop as its fold ends; dropped
  // here as it's closed where that didn't, once.
  const moving = (by) => (init, f, unwind, ends) => {
    const outer = once(ends);
    if (moved === "alone") return by(init, f, unwind, outer, own);
    const both = moved === "before" ? () => $closedAfter(own, outer) : () => $closedAfter(outer, own);
    return by(init, f, unwind, once(both));
  };
  const fold = holding.fold;
  holding.fold = moving((init, f, unwind, ends, alone) => {
    if (alone === undefined || it.fold === undefined) return fold(init, f, unwind, ends);
    // A `flat_map`'s: its fold drops `ends` after its source's, which it
    // gives its function to drop.
    try {
      return it.fold(init, f, unwind, ends, alone);
    } finally {
      $closedAfter(ends, () => holding.return());
    }
  });
  if (holding.rfold !== undefined) {
    const rfold = holding.rfold;
    holding.rfold = moving((init, f, unwind, ends) => rfold(init, f, unwind, ends));
  }
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
  it.ownNth = true;
  it.ownNthBack = true;
  it.ownLast = true;
  it.dropItem = drop;
  return it;
}
