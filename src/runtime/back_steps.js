// What a double-ended iterator does from its back, of a JS iterator that
// knows its back, a `$iter` or a `$iterator` given its `nextBack`, as Rust's
// `next_back` steps it (ADR 0429).

// `it.nth_back(n)`: its `n`th item from the back, each before it stepped
// past, and dropped, `drop`, where items have a destructor, or `undefined`,
// `None`; of items that could look like `None`, its `Some`, `$some`, given
// as `found` (ADR 0051). Its own `nth_back`, where it reaches the crate's
// (#220), or that drops what it passes over itself, a `Vec`'s `IntoIter`'s
// as one slice, front to back, as std's `advance_back_by` does
// (`$slicing`, #234); else std's default, each dropped as it's passed, a
// `Drain`'s and a B-tree's (#704).
function $nthBack(it, n, found = (item) => item, drop) {
  if (it.nthBack !== undefined) {
    const step = it.nthBack(n, drop);
    return step.done ? undefined : found(step.value);
  }
  // Std's default `nth_back`: its `advance_back_by(n)`, its own where it
  // has one, a compiled flatten's, then the next from its back (#2123).
  if (it.advanceBackBy !== undefined) {
    if (it.advanceBackBy(n, drop) > 0) return undefined;
    const step = it.back();
    return step.done ? undefined : found(step.value);
  }
  for (let step = it.back(); !step.done; step = it.back()) {
    if (n-- === 0) return found(step.value);
    drop?.(step.value);
  }
  return undefined;
}

// `it.nth_back(n)` of items that look like `None`: its `Some` boxed (ADR
// 0051), as a dictionary's of a type parameter's value gives it (#2134).
function $nthBackSome(it, n) {
  return $nthBack(it, n, $some);
}

// `it.advance_back_by(m)`, as std's own bodies call it, given as `$nthBack`
// is: its own, `advanceBackBy`, a `Vec`'s as one slice (`$slicing`, #234);
// else each `next_back`, given to `drop`, a `Drain`'s (#704). `Ok(())`, or
// `Err` of how many there weren't (ADR 0920).
function $advanceBackBy(it, m, _found, drop) {
  let left = m;
  if (it.advanceBackBy !== undefined) {
    left = it.advanceBackBy(m, drop);
  } else {
    for (; left > 0; left--) {
      const step = it.back();
      if (step.done) break;
      drop?.(step.value);
    }
  }
  return left === 0 ? { TAG: "Ok" } : { TAG: "Err", _0: left };
}

// `it.rfold(init, f)`: `f` of each item from the back, given what it gave;
// its own `rfold`, where it reaches the crate's (#220). Consuming it, it
// closes it, as Rust's `rfold` drops what it takes at its end (#206), or
// as it unwinds. Given `unwind`, the drop of an accumulator a stage's panic
// may strand, as `$foldDropping`'s: passed on to its own `rfold`, or run
// where a step throws, before what's left is closed (#423).
function $foldBack(it, init, f, unwind, ends) {
  if (it.rfold !== undefined) return it.rfold(init, f, unwind, ends);
  return $foldBy(it, "back", init, f, unwind, ends);
}

// `it.rfind(p)`: the first item from the back `p` holds of, or `undefined`;
// `found` and `drop`, of each it passes over, as `$nthBack`'s. Std's, its
// `try_rfold`, where that reaches the crate's own (#220). Each item asked
// by `$kept`, as `find`'s are, `$findDropping`'s: the one `p` throws on, a
// panic, dropped as it unwinds, as std's `check` closure holds it
// (`double_ended.rs:419`, #738).
function $findBack(it, p, found = (item) => item, drop) {
  if (it.tryRfold !== undefined) {
    let hit;
    const hits = it.tryRfold(false, (_, x) => $kept(p, drop, x) && ((hit = x), true), (hits) => hits);
    return hits ? found(hit) : undefined;
  }
  for (let step = it.back(); !step.done; step = it.back()) {
    if ($kept(p, drop, step.value)) return found(step.value);
  }
  return undefined;
}

// What an adapter stepped from its back asks of what it steps, as std's
// do (#220): its own `try_rfold`, where it reaches the crate's, else
// std's, of its `next_back`, stopped where `broke` holds of what it's made.
// Given `unwind`, the drop of an accumulator a step's panic strands, passed
// on, or run where a step throws, as `$Steps`' `tryFold` does (#423, rd767).
// And `ends`, what drops with `f`, passed on, its caller's to drop after
// where std's default holds the accumulator in its body (S2, #768).
function $tryRfoldOf(it, init, f, broke, unwind, ends) {
  if (it.tryRfold !== undefined) return it.tryRfold(init, f, broke, unwind, ends);
  let folded = init;
  const back = () => (unwind === undefined ? it.back() : $staged(() => it.back(), undefined, undefined, folded, unwind));
  for (let step = back(); !step.done; step = back()) {
    folded = f(folded, step.value);
    if (broke(folded)) return folded;
  }
  return folded;
}

// And its `nth_back(k)`, a step: its own, where it reaches the crate's,
// else std's, `k` stepped past from its back, each dropped, `drop`, then
// the next.
function $nthBackOf(it, k, drop) {
  if (it.nthBack !== undefined) return it.nthBack(k);
  if (it.advanceBackBy !== undefined) {
    if (it.advanceBackBy(k, drop) > 0) return { done: true, value: undefined };
    return it.back();
  }
  for (; k > 0; k--) {
    const step = it.back();
    if (step.done) return step;
    drop?.(step.value);
  }
  return it.back();
}

// `it` closed, `return()`, where it can be, as an adapter closed closes
// what it steps (ADR 0465): done.
function $closed(it) {
  it.return?.();
  return { done: true, value: undefined };
}

// A map of a JS iterator that knows its back or its length, `it.map(f)`,
// which knows them too, as Rust's `Map` is double-ended and exact-sized
// where its source is: each of its source's items, `f`'s, from either end
// (ADRs 0455, 0456). Closing it closes its source (ADR 0465).
function $mapEnds(it, f) {
  const mapped = (step) => (step.done ? step : { done: false, value: f(step.value) });
  const map = new $Steps(
    () => mapped(it.next()),
    () => $closed(it),
  );
  if (it.back !== undefined) map.back = () => mapped(it.back());
  if (it.len !== undefined) map.len = () => it.len();
  // Std's `Map::size_hint`, its source's (map.rs:111, #733).
  map.sizeHint = () => $sizeHint(it);
  // Std's `Map::fold`, `try_fold`, `rfold` and `try_rfold`, its source's of
  // each mapped, the crate's own where it reaches it (#220, rd762).
  // Where `f` throws, a panic, the accumulator dropped by `unwind` (#423).
  const folding = (g, unwind) =>
    unwind === undefined ? (a, x) => g(a, f(x)) : (a, x) => g(a, $staged(f, x, undefined, a, unwind));
  $forward(map, it, { fold: "fold", tryFold: "tryFold", rfold: "rfold", tryRfold: "tryRfold" }, folding);
  map.from = [it];
  return map;
}

// Std's `Copied` of `it`, each item copied by `f`: its map, `$mapEnds`,
// that passes on what std's does too, its source's `nth`, `advance_by`,
// `last`, `count` and `advance_back_by`, each the crate's own where it
// writes one, where a map's are its fold's (copied.rs, #2122). Its items
// are `Copy`: nothing it passes over has a drop.
function $copied(it, f) {
  const copied = $mapEnds(it, f);
  const mapped = (step) => (step.done ? step : { done: false, value: f(step.value) });
  $forward(copied, it, {
    nth: it.nth === undefined ? undefined : (k) => mapped(it.nth(k)),
    last: it.last === undefined ? undefined : () => mapped(it.last()),
    advanceBy: "advanceBy",
    count: "count",
    advanceBackBy: "advanceBackBy",
  });
  return $ownsAs(copied, it, "nth", "last");
}

// Std's `Cloned` of what generic code was given, of items whose clone is
// the item: `it` itself where none of the steps std's `Cloned` leaves to
// its fold, `nth`, `last`, `count` and `nth_back`, is its own, so an
// array's is still read by index (ADR 0975); else its map, `$mapEnds`,
// which asks its source's folds alone, as std's `Cloned` does (#2122).
function $cloned(it) {
  if (!["nth", "last", "count", "nthBack"].some((step) => it[$ownStep(step)])) return it;
  return $mapEnds(it, (item) => item);
}

// Std's `Copied` of what generic code was given, of items whose copy is
// the item: `it` itself unless its `nth_back` is its own, which std's
// `Copied` doesn't pass on, its default its source's `advance_back_by`;
// else `$copied` of it, which passes on each step std's `Copied` does
// (#2130). A clone of it, `$givenClone`'s, is one of a clone of `it`.
function $copiedGiven(it) {
  if (!it[$ownStep("nthBack")]) return it;
  const copied = $copied(it, (item) => item);
  copied.cloneBy = (clone) => $copiedGiven($givenClone(it, clone));
  return copied;
}

// `it.step_by(step)`, as Rust's `StepBy` steps: its first item, then
// each `step`th, a skip stopped by a `None`, which it gives, and stepped on
// after, as std's `nth` is; each passed over dropped, `drop`, where items
// have a destructor (ADR 0465). Closing it closes its source. Its `nth`,
// `fold` and `tryFold` are std's: its first item its source's `next`, then
// each its source's `nth` (#182).
function $stepBy(it, step, drop) {
  const skip = (k, by) => {
    // From the front, its source's `nth`, as std's `StepBy` steps it, the
    // crate's own where it writes one (#162); from the back, its `nth_back`,
    // as std's `next_back` asks it (#245).
    // Given the items' drop, which an `ownNth` with none of its own, a
    // `take`'s past its end, drops by (#289).
    if (by === "next" && (drop === undefined || it.ownNth) && it.nth !== undefined) return it.nth(k, drop);
    // Its own only where it drops what it passes itself, a `Vec`'s as one
    // slice (`ownNthBack`); else each passed dropped by `drop`, as std's
    // default `nth_back` drops each `next_back` gives (#823).
    if (by === "back" && (drop === undefined || it.ownNthBack) && it.nthBack !== undefined) return it.nthBack(k);
    for (; k > 0; k--) {
      const at = it[by]();
      if (at.done) return at;
      drop?.(at.value);
    }
    return it[by]();
  };
  const stepped = new $Steps(
    () => {
      const at = skip(stepped.first ? 0 : step - 1, "next");
      stepped.first = false;
      return at;
    },
    () => $closed(it),
  );
  stepped.first = true;
  stepped.skip = skip;
  // Its size hint, std's `StepBy`'s of its source's: before its first item
  // is taken, that one and each `step`th after; then each `step`th.
  stepped.sizeHint = () => {
    const [low, high] = $sizeHint(it);
    const left = (n) => (stepped.first ? (n === 0 ? 0 : 1 + Math.floor((n - 1) / step)) : Math.floor(n / step));
    return [left(low), high == null ? undefined : left(high)];
  };
  // Its `nth` its source's, which drops what it passes over itself, a
  // reversed `Vec`'s as one slice (#234).
  $ownsAs(stepped, it, "nth");
  const nth = (n) => {
    if (stepped.first) {
      stepped.first = false;
      const head = it.next();
      // Std's `spec_nth`: `self.iter.next()?`, so a source that's done
      // there is asked nothing more, one that isn't fused too (#2124).
      if (n === 0 || head.done) return head;
      drop?.(head.value);
      n -= 1;
    }
    return skip((n + 1) * step - 1, "next");
  };
  // Where a step throws, a stage's panic, what it folded dropped by
  // `unwind` (#423).
  // Its first `next`, while std's frame holds only its parameters, drops
  // what drops with `f`, `ends`, first (`$stepHolding`, #768).
  const tryFold = (init, f, broke, unwind, ends) => {
    let folded = init;
    const stepping = (run) => (unwind === undefined ? run() : $staged(run, undefined, undefined, folded, unwind));
    if (stepped.first) {
      stepped.first = false;
      const head = $stepHolding(() => it.next(), folded, unwind, ends);
      if (head.done) return folded;
      folded = f(folded, head.value);
      if (broke(folded)) return folded;
    }
    const next = () => stepping(() => skip(step - 1, "next"));
    for (let at = next(); !at.done; at = next()) {
      folded = f(folded, at.value);
      if (broke(folded)) return folded;
    }
    return folded;
  };
  // Its `fold`, its `try_fold` never stopped, then closed, as `fold(self)`
  // drops what it has left at its end (#242), and though it throws.
  // What drops with `f`, `ends`, first (#734): its first step's, or at
  // its end. Held, so what it steps is closed at its end, not inside the
  // step that finds it done, where a panic of its drop would drop the
  // accumulator, which std's fold has made its result (ADR 1092 S9, #845).
  const fold = (init, f, unwind, ends) => {
    stepped.hold();
    const folded = $consumed(stepped, () => stepped.tryFold(init, f, () => false, unwind, ends), ends);
    $closedAfter(ends, () => stepped.return());
    return folded;
  };
  $forward(stepped, it, { nth, tryFold, fold });
  stepped.from = [it];
  // Std's `StepBy::len`, its exact hint, as std's `StepBy` writes no `len`:
  // of any source, which generic code asks where its bound is
  // `ExactSizeIterator`, whatever form made it (#2193, #2194).
  stepped.len = () => stepped.sizeHint()[0];
  return stepped;
}

// `it.step_by(step)` of a JS iterator that knows its back and its length,
// which knows them too, as Rust's `StepBy` is double-ended and exact-sized
// where its source is exact-sized: `$stepBy`'s from the front; from the
// back, the last of those, found by the source's length, as std's
// `next_back_index` finds it; its length its exact hint (#718). Of items with
// a destructor, each passed over dropped by `drop`, `$stepBy`'s, where its
// source drops none itself (#823).
function $stepByEnds(it, step, drop) {
  const stepped = $stepBy(it, step, drop);
  // Std's `next_back_index`: how many its back passes over first.
  const backIndex = () => {
    const rem = it.len() % step;
    return stepped.first ? (rem === 0 ? step - 1 : rem - 1) : rem;
  };
  stepped.back = () => stepped.skip(backIndex(), "back");
  // Std's `nth_back(n)`: its source's, past `n` steps and its back's index,
  // one call, the crate's own where it writes one (#245).
  // Std's `rfold` and `try_rfold` (`spec_rfold`, `spec_try_rfold`): its back
  // once, which asks its source's length, then each its source's
  // `nth_back(step - 1)`, which asks it no more (#834). Where a step throws,
  // a stage's panic, what it folded dropped by `unwind`; its first, while
  // std's frame holds only its parameters, drops what drops with `f`,
  // `ends`, first, as its `try_fold`'s does (#423, #768).
  const tryRfold = (init, f, broke, unwind, ends) => {
    let folded = init;
    const stepping = (run) => (unwind === undefined ? run() : $staged(run, undefined, undefined, folded, unwind));
    const last = $stepHolding(() => stepped.back(), folded, unwind, ends);
    if (last.done) return folded;
    folded = f(folded, last.value);
    if (broke(folded)) return folded;
    const back = () => stepping(() => stepped.skip(step - 1, "back"));
    for (let at = back(); !at.done; at = back()) {
      folded = f(folded, at.value);
      if (broke(folded)) return folded;
    }
    return folded;
  };
  // Its `rfold`, its `try_rfold` never stopped, then closed, as
  // `rfold(self)` drops what it has left at its end, and though it throws,
  // as its `fold` is, held as it is (#845).
  const rfold = (init, f, unwind, ends) => {
    stepped.hold();
    const folded = $consumed(stepped, () => tryRfold(init, f, () => false, unwind, ends), ends);
    $closedAfter(ends, () => stepped.return());
    return folded;
  };
  $forward(stepped, it, { nthBack: (n) => stepped.skip(n * step + backIndex(), "back"), tryRfold, rfold });
  // Its `nth_back` its source's, which drops what it passes over itself, a
  // `Vec`'s as one slice, as a `step_by` over it asks (#834).
  $ownsAs(stepped, it, "nthBack");
  return stepped;
}

// The `Some`s of `it`, whose items may be `None`: std's `FilterMap`'s, a
// flatten of `Option`s' or a `flat_map` to them, made in place over a
// `$Steps` whose `size_hint` may run the crate's code (ADR 0515). A filter
// of what's there, `$filterEnds`' where `it` knows its back, but its own
// `count` neither `Filter::count`, which asks its source's hint first, nor
// any: std's `FilterMap` writes none and `Flatten`'s folds its `Option`s,
// each its source's `fold` (#2199). Of a JS iterator generic code was
// given that knows no back, JS's own `filter`, which has no `count` to ask.
function $somes(it) {
  const present = (item) => item != null;
  if (it.back === undefined && !(it instanceof $Steps)) return it.filter(present);
  const kept = it.back !== undefined ? $filterEnds(it, present) : it.filter(present);
  $forward(kept, it, { count: (drop, ends) => $Steps.prototype.count.call(kept, drop, ends) });
  return kept;
}

// The items of `it`'s `Option`s, std's `Flatten` of them or a `flat_map` to
// them, made in place where the source's steps may reach the crate's own
// `nth` or `nth_back`, or its hint the crate's code: std's `FlattenCompat`
// over one-shot inner iterators (flatten.rs:848). Its folds are a filter's
// of the `Some`s, `$somes`', each its source's, the function moved into it
// as std's `flatten_one(fold)` is (#734); its count and last their fold.
// But its `next` and `next_back` are its source's, until one gives a
// `Some`, where a filter's are its source's `try_fold`, so a `skip(1)`
// between asks the crate's `nth(1)`, not `nth(0)` and a `next` (#2205);
// its source fused, as std's `Fuse` holds it, stepped no more once it's
// given `None`. Its hint std's one-shot one (flatten.rs:864): none to its
// source's upper bound, none at all once it's ended. Of a JS iterator
// generic code was given that knows no back, JS's own `filter`, which
// steps it by `next`.
function $someItems(it) {
  const kept = $somes(it);
  if (!(kept instanceof $Steps)) return kept;
  let ended = false;
  const step = (by) => () => {
    for (;;) {
      if (ended) return { done: true, value: undefined };
      const at = it[by]();
      if (at.done) ended = true;
      if (at.done || at.value != null) return at;
    }
  };
  kept.next = step("next");
  // From its back the same, its `advance_back_by` its source's `try_rfold`,
  // as its `advance_by` is its `try_fold`, `$Steps`' (flatten.rs:948).
  if (kept.back !== undefined) {
    kept.back = step("back");
    $forward(kept, it, {
      advanceBackBy: (m, drop) => (m === 0 ? 0 : kept.tryRfold(m, (k, x) => (drop?.(x), k - 1), (k) => k === 0)),
    });
  }
  kept.sizeHint = () => (ended ? [0, 0] : [0, $sizeHint(it)[1]]);
  return kept;
}

// A filter of a JS iterator that knows its back, `it.filter(p)`, which
// knows it too, as Rust's `Filter` is double-ended where its source is:
// each of its source's items `p` keeps, from either end, `p` asked of each
// as it's reached (ADR 0526), and each it doesn't dropped by `drop`, of
// items with a destructor (ADR 0871). Closing it closes its source (ADR 0465, #225).
function $filterEnds(it, p, drop) {
  // Each item `p` keeps, the one it was asked of dropped where it throws,
  // a panic, as std's `Filter` holds it as it unwinds (#423), and each it
  // doesn't by `drop`: `$kept`'s. Its step, std's `Filter::next`, its
  // source's `find`, a `try_fold`: `$Steps`' own `filter`'s, which reaches
  // its source's own, a `rev()`'s of a `take(n)`, which trims its back by
  // its `try_rfold`, not by its `next_back` (#1226).
  const front = $filterDropping(it, p, drop);
  const filter = new $Steps(
    () => front.next(),
    () => $closed(it),
  );
  // Std's `Filter::next_back`, its source's `rfind`, `$findBack`'s: its
  // `try_rfold` where that reaches the crate's own (#220), else its back.
  if (it.back !== undefined || it.tryRfold !== undefined) {
    filter.back = () => $findBack(it, p, (value) => ({ done: false, value }), drop) ?? { done: true, value: undefined };
  }
  // Std's `Filter::fold`, `try_fold`, `rfold` and `try_rfold`, its source's
  // of each kept (#220, #625), the accumulator held as `$kept` says (#423,
  // #651); from the back, its source's own or std's default of its back,
  // a compiled `Zip`'s say, `$foldBack`'s and `$tryRfoldOf`'s; given
  // what drops with `f`, `ends`, passed on (#734).
  const folding = (f, unwind) => (a, x) => ($kept(p, drop, x, undefined, a, unwind) ? f(a, x) : a);
  const back = it.back !== undefined;
  $forward(
    filter,
    it,
    {
      fold: "fold",
      tryFold: "tryFold",
      rfold: back ? (init, f, unwind, ends) => $foldBack(it, init, folding(f, unwind), unwind, ends) : undefined,
      tryRfold: back
        ? (init, f, broke, unwind, ends) => $tryRfoldOf(it, init, folding(f, unwind), broke, unwind, ends)
        : undefined,
    },
    folding,
  );
  // Std's `Filter::size_hint`, none to its source's upper bound, and its
  // `count`, its source's hint asked first (filter.rs:118, 135, #733).
  filter.sizeHint = () => $upperOnly(it);
  $forward(filter, it, {
    count: (drop, ends) => {
      $sizeHint(it);
      return $Steps.prototype.count.call(filter, drop, ends);
    },
  });
  // Its own, as `$Steps`' `filter`'s is (#2212).
  $owns(filter, "count");
  filter.from = [it];
  return filter;
}

// `it.enumerate()`, Rust's `Enumerate`, explicit (ADR 0542): what it
// steps, `iter`, and how many it's given, `count`, kept, which its size
// hint, its `Debug` and a clone read. Each item and its index, `[i, x]`,
// counted from the front; one from the back is where it is, the count so
// far and how many were left before it, as Rust's `next_back` counts it
// (ADR 0528). Its back and its length are its source's, asked as they're
// asked: a use site may give a source its own, `$knowing`'s. Closing it
// closes what it steps.
function $enumerate(iter) {
  const pair = (step, at) => (step.done ? step : { done: false, value: [at, step.value] });
  const enumerated = new $Steps(
    () => {
      const step = enumerated.iter.next();
      return step.done ? step : pair(step, enumerated.count++);
    },
    () => $closed(enumerated.iter),
  );
  enumerated.iter = iter;
  enumerated.count = 0;
  enumerated.sizeHint = () => $sizeHint(enumerated.iter);
  // Std's `Enumerate::next_back`: its source's, then where it is, the count
  // so far and how many are left after it, asked once it's stepped (#738).
  enumerated.back = () => {
    const step = enumerated.iter.back();
    return step.done ? step : pair(step, enumerated.count + enumerated.iter.len());
  };
  enumerated.len = () => enumerated.iter.len();
  // Std's `Enumerate::count`: its source's, which `$count` asks by this
  // name, `count` being std's field of how many it's given (ADR 0988); so
  // a reversed `Vec`'s take drops what it counts as one slice, front to
  // back (#493). Given a pair's drop, of the item it counts.
  const countItems = (drop) => $count(enumerated.iter, drop && ((x) => drop([0, x])));
  // Std's `fold` and `try_fold`: its source's, each item counted. Each of
  // its folds gives its source `unwind`, the drop of an accumulator a
  // step's panic strands, as std's passes its closure on (#423, rd767).
  const fold = (init, f, unwind, ends) =>
    enumerated.iter.fold(init, (a, x) => f(a, [enumerated.count++, x]), unwind, ends);
  const tryFold = (init, f, broke, unwind, ends) =>
    $lent(enumerated.iter).tryFold(init, (a, x) => f(a, [enumerated.count++, x]), broke, unwind, ends);
  // Std's `rfold`, `try_rfold` and `nth_back`: its source's, each counted
  // from where it is, the crate's own where they reach it (#220).
  const rfold = (init, f, unwind, ends) => {
    let at = enumerated.count + enumerated.iter.len();
    return $foldBack(enumerated.iter, init, (a, x) => f(a, [--at, x]), unwind, ends);
  };
  const tryRfold = (init, f, broke, unwind, ends) => {
    let at = enumerated.count + enumerated.iter.len();
    return $tryRfoldOf(enumerated.iter, init, (a, x) => f(a, [--at, x]), broke, unwind, ends);
  };
  // Its `nth_back`: its source's, then where it is, its length asked only
  // of an item it found, as std's `?` returns first (#834).
  const nthBack = (k, drop) => {
    const step = $nthBackOf(enumerated.iter, k, drop && ((x) => drop([0, x])));
    return step.done ? step : pair(step, enumerated.count + enumerated.iter.len());
  };
  // Std's `nth(k)`: its source's, counted `k` more, the crate's own where
  // it writes one (#182); its own where its source's is, which drops what
  // it passes itself, a reversed `Vec`'s, as a `step_by` over it asks (#277).
  // Its source's is given the items' drop, a pair's of the item it counts,
  // by which a `take` past its end drops what its source's `nth` gives
  // back (#492).
  $ownsAs(enumerated, iter, "nth");
  // And its `nth_back`, its source's, its own where its source's is, which
  // a `step_by` over it asks from its back (#834).
  $ownsAs(enumerated, iter, "nthBack");
  // Its `count`, `countItems`, its source's, the crate's own where it
  // writes one, which a `skip` of it asks (#671).
  $ownsAs(enumerated, iter, "count");
  const nth = (k, drop) => {
    const step = enumerated.iter.nth(k, drop && ((x) => drop([0, x])));
    if (step.done) return step;
    const at = enumerated.count + k;
    enumerated.count = at + 1;
    return { done: false, value: [at, step.value] };
  };
  // Std's `advance_by(m)`: its source's, given the item's drop of a pair's,
  // counted as far as it went, so a `take`'s `count` of a reversed `Vec`'s
  // drops what it passes as one slice, front to back (#654).
  const advanceBy = (m, drop) => {
    const left = enumerated.iter.advanceBy(m, drop && ((x) => drop([0, x])));
    enumerated.count += m - left;
    return left;
  };
  // Std's `advance_back_by(m)`: its source's, given the item's drop of a
  // pair's, where it has one, so a reversed `take`'s `count` of a `Vec`'s
  // drops what it passes as one slice, front to back (#2120). It counts
  // from the front, so nothing it counts moves.
  const advanceBackBy = (m, drop) => {
    const advanced = $advanceBackBy(enumerated.iter, m, undefined, drop && ((x) => drop([0, x])));
    return advanced.TAG === "Ok" ? 0 : advanced._0;
  };
  // Each but its `count` and `try_fold` where its source has it.
  $forward(enumerated, iter, {
    countItems,
    advanceBy: iter.advanceBy === undefined ? undefined : advanceBy,
    fold: iter.fold === undefined ? undefined : fold,
    tryFold,
    nth: iter.nth === undefined ? undefined : nth,
  });
  // Its back steps std's wherever it steps from its back: how many are
  // left asked once, then its source's own, or std's of its back, an
  // `rfind`'s (#738, #2140).
  $forwardBack(enumerated, iter, { advanceBackBy, rfold, tryRfold, nthBack });
  return enumerated;
}

// A flatten of a JS iterator that knows its back, `it.flatMap(each)`, each
// of whose items `each` makes what knows its back too, an array's `$iter`:
// which knows its own, as Rust's `Flatten` and `FlatMap` are double-ended
// where their source and its items are. From the front, the front inner's
// items, then the next of the source's, then, its source done, the back
// inner's; from the back, the same the other way, as std's `FlattenCompat`
// steps them, its source fused. Closing it closes what it holds in
// `FlattenCompat`'s field order, its source, then the inner it steps from
// the front, then the one from the back, each though an earlier one's
// drop throws, a panic, as Rust's drop glue drops each field as it unwinds
// (#705). Of a function value given, `flat_map(pick())`, each item what
// `knowing` makes of what `each` gives, knowing its back (#1235).
function $flattenEnds(it, each, knowing) {
  const made = knowing === undefined ? each : (x) => knowing(each(x));
  const done = { done: true, value: undefined };
  const inner = { next: undefined, back: undefined };
  // Its source, once done from either end, stays done, as std's `Fuse` of
  // it does.
  let ended = false;
  const step = (by, own, other) => () => {
    for (;;) {
      if (inner[own] !== undefined) {
        const at = inner[own][by]();
        if (!at.done) return at;
        inner[own] = undefined;
      }
      const outer = ended ? done : it[by]();
      if (outer.done) {
        ended = true;
        if (inner[other] === undefined) return done;
        const at = inner[other][by]();
        if (at.done) inner[other] = undefined;
        return at;
      }
      inner[own] = $iter(made(outer.value));
    }
  };
  const taken = (own) => {
    const held = inner[own];
    inner[own] = undefined;
    held?.return?.();
  };
  // Its source, then the function a `$holding` gives it, then its inner
  // iterators, as `FlatMap`'s drop glue drops its `Map` first (#2241).
  const close = () =>
    $inTurn(
      () => it.return?.(),
      () => flat.functionDrop?.(),
      () => taken("next"),
      () => taken("back"),
    );
  const flat = new $Steps(step("next", "next", "back"), close);
  flat.back = step("back", "back", "next");
  flat.from = [it];
  // Std's `FlattenCompat::size_hint`: what's left of the inner iterators
  // it's in, front and back, then its source's, fused, done once it's
  // ended (`$flatHint`, #1892).
  flat.sizeHint = () => $flatHint(inner.next, ended ? undefined : it, inner.back);
  // Std's `FlattenCompat::fold`: the front inner's rest, then its source's
  // `fold` of each item's inner, then the back inner's rest; its `rfold`
  // the same from the back. Each is its source's and each inner's own step,
  // the crate's where it writes one, which a `rev()`'s `rfold` and `fold`
  // ask too (#621, #649). What a fold hasn't reached yet, its source and
  // its other inner, stays its own, closed in their order as what consumes
  // it unwinds (#705).
  const front = (stepped, init, f, unwind, ends) =>
    typeof stepped.fold === "function" ? stepped.fold(init, f, unwind, ends) : $foldDropping(stepped, init, f, unwind, ends);
  // What drops with `f`, `ends`, after its parts, as `FlattenCompat::fold`
  // folds them by `&mut fold`, dropped as it returns (#734).
  // Where the function that makes each inner throws, a panic, the
  // accumulator it strands dropped by `unwind`, as std's stage of it does
  // (#744). The function a `$holding` gives it to drop, `alone`, its
  // source's fold drops, as `Map::fold` moves it into its source's (#2241).
  const folding = (fold, own, other) => (init, f, unwind, ends, alone) => {
    const inners = (stepped, a) => fold(stepped, a, f, unwind);
    const staged = unwind === undefined ? (x) => made(x) : (x, a) => $staged(made, x, undefined, a, unwind);
    return $endsAfter(ends, () => {
      let folded = init;
      const first = inner[own];
      inner[own] = undefined;
      if (first !== undefined) folded = inners(first, folded);
      if (!ended) {
        ended = true;
        folded = fold(it, folded, (a, x) => inners($iter(staged(x, a)), a), unwind, alone);
      }
      const last = inner[other];
      inner[other] = undefined;
      return last === undefined ? folded : inners(last, folded);
    });
  };
  // Std's `FlattenCompat::count`: the front inner's own `count`, then its
  // source's fold of each item's inner's, then the back inner's, the
  // crate's where it writes one, each given the items' drop (#736). What
  // takes it closes it as one throws, a panic, what it still holds.
  // What drops with a `$holding`'s function, `ends`, its source's fold
  // drops, as `$flatMap`'s `count` does (#2224).
  const count = (drop, ends) => {
    let counted = 0;
    const first = inner.next;
    inner.next = undefined;
    if (first !== undefined) counted = $count(first, drop);
    if (!ended) {
      ended = true;
      counted = front(it, counted, (n, x) => n + $count($iter(made(x)), drop), undefined, ends);
    }
    const last = inner.back;
    inner.back = undefined;
    return last === undefined ? counted : counted + $count(last, drop);
  };
  // Std's `FlattenCompat::try_fold`, as `$flatMap`'s: the front inner's
  // rest, then its source's `try_fold` of each item's inner, then the back
  // inner's, each broken where `broke` says, the inner it broke in kept to
  // step on from; its source, done, fused. A step its sibling has, so what
  // stops early, a collect into an `Option` or a `find`, steps it as Rust's
  // does, not by its `next` (#2051).
  const tryFold = (init, g, broke, unwind, ends) => {
    let folded = init;
    if (inner.next !== undefined) {
      folded = $lent(inner.next).tryFold(folded, g, broke, unwind);
      if (broke(folded)) return folded;
      inner.next = undefined;
    }
    if (!ended) {
      folded = $lent(it).tryFold(
        folded,
        (a, x) => {
          inner.next = $iter(unwind === undefined ? made(x) : $staged(made, x, undefined, a, unwind));
          const stepped = $lent(inner.next).tryFold(a, g, broke, unwind);
          if (!broke(stepped)) inner.next = undefined;
          return stepped;
        },
        broke,
        unwind,
      );
      if (broke(folded)) return folded;
      ended = true;
    }
    if (inner.back !== undefined) {
      folded = $lent(inner.back).tryFold(folded, g, broke, unwind);
      if (broke(folded)) return folded;
      inner.back = undefined;
    }
    return folded;
  };
  // Std's `FlattenCompat::try_rfold`, `tryFold`'s from the back: the back
  // inner's rest, then its source's `try_rfold` of each item's inner, then
  // the front inner's (#2123). What drops with `g`, `ends`, isn't passed
  // on, as `tryFold`'s isn't: its caller drops it after (#768).
  const tryRfold = (init, g, broke, unwind, ends) => {
    let folded = init;
    if (inner.back !== undefined) {
      folded = $tryRfoldOf(inner.back, folded, g, broke, unwind);
      if (broke(folded)) return folded;
      inner.back = undefined;
    }
    if (!ended) {
      folded = $tryRfoldOf(
        it,
        folded,
        (a, x) => {
          inner.back = $iter(unwind === undefined ? made(x) : $staged(made, x, undefined, a, unwind));
          const stepped = $tryRfoldOf(inner.back, a, g, broke, unwind);
          if (!broke(stepped)) inner.back = undefined;
          return stepped;
        },
        broke,
        unwind,
      );
      if (broke(folded)) return folded;
      ended = true;
    }
    if (inner.next !== undefined) {
      folded = $tryRfoldOf(inner.next, folded, g, broke, unwind);
      if (broke(folded)) return folded;
      inner.next = undefined;
    }
    return folded;
  };
  // Std's `FlattenCompat::advance_by` and `advance_back_by`: the inner it's
  // in at that end's own, then its source's `try_fold` or `try_rfold` of
  // each item's inner's own, the one it stops in kept to step on from, then
  // the other end's inner's; how many there weren't. Its `nth` and
  // `nth_back` are std's defaults of them (#2123).
  const advancing = (own, other, advance, fold) => (m, drop) => {
    let left = m;
    if (inner[own] !== undefined) {
      left = advance(inner[own], left, drop);
      if (left === 0) return 0;
      inner[own] = undefined;
    }
    if (!ended) {
      let stopped = false;
      left = fold(it, left, (k, x) => {
        inner[own] = $iter(made(x));
        const rest = advance(inner[own], k, drop);
        stopped = rest === 0;
        if (!stopped) inner[own] = undefined;
        return rest;
      });
      if (stopped) return 0;
      ended = true;
    }
    if (inner[other] !== undefined) {
      left = advance(inner[other], left, drop);
      if (left === 0) return 0;
      inner[other] = undefined;
    }
    return left;
  };
  const advanceBy = advancing("next", "back", $advanceByOf, (source, init, f) =>
    $lent(source).tryFold(init, f, (k) => k === 0),
  );
  const advanceBackBy = advancing(
    "back",
    "next",
    (inner, m, drop) => {
      const advanced = $advanceBackBy(inner, m, undefined, drop);
      return advanced.TAG === "Ok" ? 0 : advanced._0;
    },
    (source, init, f) => $tryRfoldOf(source, init, f, (k) => k === 0),
  );
  $forward(flat, it, {
    fold: folding(front, "next", "back"),
    rfold: folding($foldBack, "back", "next"),
    tryFold,
    tryRfold,
    count,
    advanceBy,
    advanceBackBy,
    nthBack: (k, drop) => (advanceBackBy(k, drop) > 0 ? { done: true, value: undefined } : flat.back()),
    last: (drop, ends) => $flatLast(flat, drop, ends),
  });
  // Its `count` its own, which what holds it passes on, as `$flatMap`'s is;
  // its `last`, each inner's own; its `nth_back`, which drops what it
  // passes itself.
  $owns(flat, "count", "last", "nthBack");
  // What std's `FlattenCompat::last` takes, as `$flatMap`'s: the inner it
  // steps from the front, its source, unless it's ended, what makes each
  // inner, and the one it steps from the back, which its fold reaches
  // last, each let go as it's taken (`$lastDropping`, #645, #2051).
  flat.inners = () => {
    const [front, back] = [inner.next, inner.back];
    inner.next = undefined;
    inner.back = undefined;
    const source = ended ? undefined : it;
    ended = true;
    return [front, source, made, back];
  };
  return flat;
}
