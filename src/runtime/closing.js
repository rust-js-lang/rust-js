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
  // (#250); how many there weren't.
  // A `Vec`'s `IntoIter`, whose `last` is its own (#648).
  const intoIter = slices && it.items !== undefined && it.nthBack === undefined && drop !== undefined;
  const slice = intoIter && drop.each !== undefined;
  const advanceBackBy = (m) => $advanceSlice(it, m, drop.each, true);
  const advanceBy = (m) => $advanceSlice(it, m, drop.each);
  if (slice) {
    closing.ownNthBack = true;
    closing.ownNth = true;
    // Each item's own drop, which what it gives on is dropped by (#261).
    closing.dropItem = drop.each;
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
  // Closed though `f` throws, a panic, as Rust's `fold` drops what's left
  // of what it took as it unwinds (#422); done with, not closed, where its
  // fold is its own, `own`, which took it and drops it as it unwinds (#423).
  const unwound = (fold, own) => {
    try {
      return fold();
    } catch (error) {
      if (own) closed = true;
      close();
      throw error;
    }
  };
  // Given `unwind`, what drops an accumulator a stage's panic strands, it
  // passes it on, so a step of the crate's own iterator that panics drops
  // a `last`'s kept item before what's left (#423, #426).
  const fold = (init, f, unwind) => {
    const folded = unwound(
      () => (it.fold !== undefined ? it.fold(init, f, unwind) : $foldDropping(it, init, (a, x) => f(a, x), unwind)),
      it.ownFold,
    );
    if (it.ownFold) closed = true;
    close();
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
  const rfold = (init, f, unwind) => {
    const folded = unwound(() => it.rfold(init, f, unwind), it.ownRfold);
    if (it.ownRfold) closed = true;
    close();
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
    nth:
      it.nth !== undefined
        ? (k, dropItem) => done(it.nth(k, dropItem))
        : slice
          ? (k) => done(advanceBy(k) > 0 ? { done: true, value: undefined } : it.next())
          : undefined,
    nthBack:
      it.nthBack !== undefined
        ? (k) => done(it.nthBack(k))
        : slice
          ? (k) => done(advanceBackBy(k) > 0 ? { done: true, value: undefined } : it.back())
          : undefined,
    fold,
    count: it.ownCount ? count : undefined,
    last: it.ownLast || intoIter ? last : undefined,
    rfold: it.rfold !== undefined ? rfold : undefined,
    tryRfold: "tryRfold",
    tryFold,
  });
  return closing;
}

// `it`, the array's items a `Vec`'s `IntoIter` owns, advanced by `m` from
// its front, or from its back, `back`: what it passes dropped as one slice,
// front to back, by `each`, as std's `advance_by` and `advance_back_by` drop
// them, the rest of it after one throws, a destructor's panic, as Rust's
// drop of a slice goes on; how many there weren't (#234, #250, #286, #670).
function $advanceSlice(it, m, each, back = false) {
  const n = Math.min(m, it.len());
  const passed = back ? it.items.slice(it.end - n, it.end) : it.items.slice(it.at, it.at + n);
  if (back) it.end -= n;
  else it.at += n;
  $dropItems(passed, each);
  return m - n;
}
