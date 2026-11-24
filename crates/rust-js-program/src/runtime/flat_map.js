// What a flatten is given of a source that isn't a `FusedIterator`, whose
// end std's `Fuse` drops it at (#2242).
const $unfused = "unfused";

// What a flatten is given of a source only a caller knows, whether it's
// fused as its callers say, `IFused`: `$unfused` where it isn't (#2242).
function $unfusedUnless(fused) {
  return fused ? undefined : $unfused;
}

// `it.flat_map(f)`, or `flatten()`, of inner iterators that hold what has a
// destructor, or give it: JS's `flatMap`, but closed as Rust drops a
// `FlatMap`, its source first, then the inner iterator it's stepping, each
// `f` made (ADR 0510, #644), its source fused, as std's is. A `$Steps`,
// not a generator, which would close both as a step throws, a panic in
// `f`: what takes it closes it, after what it holds itself, a `last`'s kept
// item, as Rust's frames unwind (#426).
// Std's `FlattenCompat::last` of `flat`: each inner iterator's own `last`,
// by its source's fold, `$lastDropping`'s of what it holds (`inners`,
// #645), a step (#2123).
// Each inner's `last` at `FlattenCompat::last`'s line of it, a `Repeat`'s
// panic's (#2488).
function $flatLast(flat, drop, ends) {
  const last = $infinitelyAt($stdSites.flattenLast[0], () => $lastDropping(flat, drop, true, ends));
  return last === undefined ? { done: true, value: undefined } : { done: false, value: $someValue(last) };
}

// Its source fused as std's `Fuse` is: one that isn't a `FusedIterator`,
// `fusing` `$unfused`, dropped at its end with the function a `$holding`
// gives it, as `Fuse` drops its `Map` at its first `None`; one that is,
// kept until it's dropped, as `Fuse`'s specialization keeps it (#2242).
function $flatMap(it, f, fusing) {
  let inner;
  // Its source done, `ended`; dropped, by its own fold or here, `dropped`.
  let ended = false;
  let dropped = false;
  const dropSource = () => {
    if (dropped) return;
    dropped = true;
    it.return?.();
  };
  // At its end, a source that isn't fused, which it doesn't hold, drops
  // itself; its function after it, as `Fuse` drops its `Map`.
  const sourceEnded = () => {
    ended = true;
    if (fusing === $unfused) flat.functionDrop?.();
  };
  const flat = new $Steps(
    () => {
      for (;;) {
        if (inner !== undefined) {
          const step = inner.next();
          if (!step.done) return step;
          inner = undefined;
        }
        if (ended) return { done: true, value: undefined };
        const outer = it.next();
        // Done, its source fused as std's `Fuse` fuses it.
        if (outer.done) {
          sourceEnded();
          return outer;
        }
        inner = $stepsOf(f(outer.value));
      }
    },
    // Its inner iterator dropped though its source's drop throws, a panic,
    // as Rust's drop glue drops each field as it unwinds; after its source,
    // the function a `$holding` gives it, as `FlatMap` drops its `Map`
    // before its inner iterators (#2241).
    () =>
      $inTurn(
        () => {
          ended = true;
          dropSource();
        },
        () => flat.functionDrop?.(),
        () => {
          const front = inner;
          inner = undefined;
          front?.return?.();
        },
      ),
  );
  // Its `fold`, std's `FlattenCompat::fold`: the rest of the one it's in,
  // then its source's own fold of each one's, as `$Steps`' `flatMap` folds,
  // so a `count` reaches the crate's own `fold` (#162). Its source's by
  // `&mut` of what drops with `g`, `ends`, which it drops after; `own`, its
  // function, `$holding`'s, moved into its source's, as its `Map` is (#710).
  const fold = (init, g, unwind, ends, own) =>
    $endsAfter(ends, () => {
      let folded = init;
      if (inner !== undefined) folded = $foldDropping(inner, folded, g, unwind);
      inner = undefined;
      if (ended) return folded;
      ended = true;
      dropped = true;
      const made = unwind === undefined ? f : (x, a) => $staged(f, x, undefined, a, unwind);
      const each = (a, x) => $foldDropping($stepsOf(made(x, a)), a, g, unwind);
      return typeof it.fold === "function" ? it.fold(folded, each, unwind, own) : $foldDropping(it, folded, each, unwind, own);
    });
  // Its `try_fold`, std's: the rest of the one it's in, then its source's
  // of each one's, the one it stops in kept for its next step.
  // What drops with `g`, `ends`, isn't passed on: std's `FlattenCompat`
  // folds its source by a closure that borrows `fold`, so its source's
  // frame never drops it; its caller drops it after (#768).
  const tryFold = (init, g, broke, unwind, ends) => {
    let folded = init;
    if (inner !== undefined) {
      folded = $lent(inner).tryFold(folded, g, broke, unwind);
      if (broke(folded)) return folded;
      inner = undefined;
    }
    if (ended) return folded;
    folded = $lent(it).tryFold(
      folded,
      (a, x) => {
        inner = $stepsOf(unwind === undefined ? f(x) : $staged(f, x, undefined, a, unwind));
        const made = $lent(inner).tryFold(a, g, broke, unwind);
        if (!broke(made)) inner = undefined;
        return made;
      },
      broke,
      unwind,
    );
    if (!broke(folded)) sourceEnded();
    return folded;
  };
  // Its `count`, std's `FlattenCompat::count`: the own `count` of the one
  // it's in, then its source's fold of each one's own, the crate's where
  // one writes it, each given the items' drop (#736). What takes it closes
  // it as one throws, a panic, its source unless it's folded it, which
  // closes itself. What drops with its function, `ends`, `$holding`'s,
  // its source's fold drops, as its `Map` is folded by value (#2224).
  const count = (drop, ends) => {
    let counted = 0;
    if (inner !== undefined) {
      const front = inner;
      inner = undefined;
      counted = $count(front, drop, $stdSites.flattenCount[0]);
    }
    if (ended) return counted;
    ended = true;
    dropped = true;
    // Each inner's at `FlattenCompat::count`'s line of it, a `Repeat`'s
    // panic's (#2488).
    const each = (n, x) => n + $count($stepsOf(f(x)), drop, $stdSites.flattenCount[0]);
    return typeof it.fold === "function"
      ? it.fold(counted, each, undefined, ends)
      : $foldDropping(it, counted, each, undefined, ends);
  };
  // Its `advance_by`, std's `FlattenCompat::advance_by`: the one it's in's
  // own, then its source's `try_fold` of each one's own, the one it stops
  // in kept to step on from; how many there weren't. Its `nth` is std's
  // default of it, `$Steps`', so each asks what std's asks (#2123).
  const advanceBy = (m, drop) => {
    let left = m;
    if (inner !== undefined) {
      left = $advanceByOf(inner, left, drop);
      if (left === 0) return 0;
      inner = undefined;
    }
    if (ended) return left;
    let stopped = false;
    left = $lent(it).tryFold(
      left,
      (k, x) => {
        inner = $stepsOf(f(x));
        const rest = $advanceByOf(inner, k, drop);
        stopped = rest === 0;
        if (!stopped) inner = undefined;
        return rest;
      },
      () => stopped,
    );
    if (!stopped) sourceEnded();
    return left;
  };
  // Held, its source held too where it's fused, so it isn't dropped at its
  // end, a `take`'s source as it's taken, but as the flatten is, as `Fuse`
  // keeps it (#2242).
  if (fusing !== $unfused) {
    const hold = flat.hold;
    flat.hold = () => {
      hold.call(flat);
      it.hold?.();
    };
  }
  $forward(flat, it, { fold, tryFold, count, advanceBy, last: (drop, ends) => $flatLast(flat, drop, ends) });
  // Its `count` its own, which what holds it passes on, a `skip()`'s, as
  // std's `Skip::count` asks its source's; and its `last`, each inner's own.
  $owns(flat, "count", "last");
  // What std's `Flatten::last` takes, its inner iterator stepped part-way,
  // its source, unless it's ended, fused as std's is, and `f`, which make
  // the rest, let go as they're taken (`$lastDropping`, #645).
  // Std's `FlattenCompat::size_hint`, of the one it's in, then its
  // source, fused.
  flat.sizeHint = () => $flatHint(inner, ended ? undefined : it);
  flat.inners = () => {
    const front = inner;
    const source = ended ? undefined : it;
    inner = undefined;
    ended = true;
    dropped ||= source !== undefined;
    return [front, source, f];
  };
  return flat;
}
