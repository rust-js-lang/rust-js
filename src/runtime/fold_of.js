// `it.fold(init, f)` of a JS iterator whose `fold` is std's adapter's over
// the crate's own, as a dictionary's `fold` is called, of the value it's of
// (#214); given `unwind`, what a stage's panic strands, `$foldDropping`'s,
// its own fold's to pass on (#423); and `ends`, what drops with `f`, which
// its fold drops before what it has left (#734).
function $foldOf(it, init, f, unwind, ends) {
  return it.fold(init, f, unwind, ends);
}

// `it.fold(init, f)` of an accumulator with a destructor that a stage's
// panic may strand, `unwind` its drop: its own `fold`, given it, which
// passes it on to each stage; else stepped by `next`, where a step's throw
// drops what's folded, then what's left is closed, as std's adapter's fold
// closure drops the accumulator it holds, then its fold what it took
// (#423). Of an array, or given no `unwind`, its `reduce`. Given `ends`,
// what drops with `f`, its own fold's to drop before what it has left,
// which its caller closes it after (#710).
function $foldDropping(it, init, f, unwind, ends) {
  if (typeof it.fold === "function" && ends !== undefined && !Array.isArray(it)) return it.fold(init, f, unwind, ends);
  if (unwind === undefined || Array.isArray(it)) return it.reduce(f, init);
  if (typeof it.fold === "function") return it.fold(init, f, unwind);
  return $foldBy(it, "next", init, f, unwind, ends);
}

// Rust's by-value `fold(self, init, f)`, `rfold`, and `reduce(self, f)`,
// its first `next` inside, `step` "fold", "rfold" or "reduce", where the
// consumer's own function drops something: `f` of each item from `init` (of
// a `reduce`, whether its `Some` is boxed); then, at its end and as it
// unwinds, in std's order (iterator.rs:2660), `unwind(acc)` where a stage's
// panic strands it, then what drops with `f`, `ends`, once, given to the
// fold it runs, which drops it before what it has left, then what's left of
// `it`, closed. The value's own drop is its caller's, after (#734).
function $consume(it, step, init, f, unwind, ends) {
  let dropped = false;
  const once = () => {
    if (!dropped) {
      dropped = true;
      ends();
    }
  };
  // Held, so what it steps closes as `$consume` closes it, after `ends`, not
  // as it's stepped to its end, a `step_by`'s or a `take`'s source's: std's
  // fold drops `self` whole after `f` (ADR 0570's statement end).
  it.hold?.();
  const close = () => it.return?.();
  const folded = $unwinds(
    () => {
      if (step === "rfold") return $foldBack(it, init, f, unwind, once);
      if (step === "fold") return $foldDropping(it, init, f, unwind, once);
      const first = it.next();
      const reduced = first.done ? undefined : $foldDropping(it, first.value, f, unwind, once);
      return init && !first.done ? $some(reduced) : reduced;
    },
    once,
    close,
  );
  $closedAfter(once, close);
  return folded;
}

// `collect()` into a `Vec` of a `TrustedLen` iterator, as std's
// `extend_trusted` does it: each item through its `fold`, std's compiled one
// where it's std's struct, a zip's `SpecFold` say, which steps no item past
// its shorter side's (#380).
//
// Where its fold throws, a panic in a closure, what it collected dropped
// by `drop`, of items with a destructor, and what's left of it closed, as
// std's unwinds, what's left first, `restFirst`, where it doesn't collect
// in place (#423). In place, what it collected is the fold's accumulator,
// std's `InPlaceDrop`, which the stage that panics drops before its fold
// drops what's left: the fold's `unwind`. In place of items with nothing to
// drop, `inPlace`, as its caller knows it (#710).
function $foldToArray(it, drop, restFirst = false, inPlace = drop !== undefined && !restFirst) {
  // A JS iterator's own helper, which has no fold, stepped by `next`.
  if (typeof it.fold !== "function") return $collectDropping(it, drop, restFirst);
  const items = [];
  const unwind = restFirst || drop === undefined ? undefined : () => $dropItems(items, drop);
  const push = (items, x) => {
    items.push(x);
    return items;
  };
  // In place, what it collected is dropped by the fold's `unwind` alone, as
  // std's `InPlaceDrop` is by the frame that holds it: not where a stage's
  // closure has moved it into what it gives, then panicked dropping an item
  // it discards, which leaks it (`$kept`, #681).
  const unwinding = $collectUnwound(it, items, unwind === undefined ? drop : undefined, restFirst);
  // In place, `inPlace`, std's `try_fold` by `&mut`, which moves no
  // adapter's function into the fold it asks, then what's left closed, an
  // adapter's function after it (#710).
  if (!inPlace || typeof it.tryFold !== "function") return $unwinds(() => it.fold(items, push, unwind), unwinding);
  // Held, so nothing it steps closes inside the fold, a `take`'s source at
  // its limit, but all of it after, at the end (#843).
  it.hold?.();
  const folded = $unwinds(() => it.tryFold(items, push, () => false, unwind), unwinding);
  // Then what its source, the `Vec` whose buffer it reuses, has left
  // dropped, where a drop of it that panics drops what it collected as it
  // unwinds, after it, as std's in-place guard does once its fold is done
  // (`InPlaceDstDataSrcBufDrop`, #843); then the adapters closed, their
  // functions, whose drop's panic leaks what it collected, as std's
  // `from_iter` has made it its result by then (rust-lang/rust#47949).
  $collectEnded($inPlaceSource(it), items, drop);
  it.return?.();
  return folded;
}

// What a collect in place of `it` reuses the buffer of, a `Vec`'s
// `IntoIter`: the source at its root, each adapter's first, `from[0]`,
// that closes.
function $inPlaceSource(it) {
  let source = it;
  while (source.from?.[0]?.return !== undefined) source = source.from[0];
  return source;
}

// `it` closed at the end of a collect into `items`, which is dropped by
// `drop` as a panic of that close unwinds, as std's `from_iter` drops the
// `Vec` it holds then (#677, #843).
function $collectEnded(it, items, drop) {
  $unwinds(
    () => it.return?.(),
    () => drop && $dropItems(items, drop),
  );
}

// `collect()` into a `Vec` of what may throw as it's stepped, a panic in a
// closure, of items with a destructor, `drop`, or from an owning source: as
// std's unwinds, what it collected dropped, then what's left of what it
// steps, closed (#422); or, `restFirst`, what's left first, where std
// collects it otherwise than in place, the fold's own dropped before the
// `Vec` (#423).
function $collectDropping(it, drop, restFirst = false) {
  const items = [];
  $unwinds(() => {
    for (let step = it.next(); !step.done; step = it.next()) items.push(step.value);
  }, $collectUnwound(it, items, drop, restFirst));
  // And at its end, as std's `collect` drops what it took then, what an
  // adapter that stops early left (ADR 0570), `$collectEnded`'s.
  $collectEnded(it, items, drop);
  return items;
}

// What a collect a panic stopped holds, the drop it runs as it unwinds,
// `$unwinds`': `items`, what it collected, by `drop`, and what's left of
// `it`, closed, in turn in std's order; what's left first, `restFirst`,
// where std's fold drops it before its `Vec` (#422, #423).
function $collectUnwound(it, items, drop, restFirst) {
  const collected = () => {
    if (drop !== undefined) $dropItems(items, drop);
  };
  const rest = () => it.return?.();
  return restFirst ? () => $inTurn(rest, collected) : () => $inTurn(collected, rest);
}
