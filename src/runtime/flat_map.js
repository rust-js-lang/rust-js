// `it.flat_map(f)`, or `flatten()`, of inner iterators that hold what has a
// destructor, or give it: JS's `flatMap`, but closed as Rust drops a
// `FlatMap`, its source first, then the inner iterator it's stepping, each
// `f` made (ADR 0510, #644), its source fused, as std's is. A `$Steps`,
// not a generator, which would close both as a step throws, a panic in
// `f`: what takes it closes it, after what it holds itself, a `last`'s kept
// item, as Rust's frames unwind (#426).
function $flatMap(it, f) {
  let inner;
  let ended = false;
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
        // Done, its source is dropped, as std's `Fuse` drops what it holds
        // at its first `None`.
        if (outer.done) {
          ended = true;
          it.return?.();
          return outer;
        }
        inner = $stepsOf(f(outer.value));
      }
    },
    // Its inner iterator dropped though its source's drop throws, a panic,
    // as Rust's drop glue drops each field as it unwinds.
    () => {
      try {
        if (!ended) {
          ended = true;
          it.return?.();
        }
      } finally {
        const front = inner;
        inner = undefined;
        front?.return?.();
      }
    },
  );
  // Its `fold`, std's `FlattenCompat::fold`: the rest of the one it's in,
  // then its source's own fold of each one's, as `$Steps`' `flatMap` folds,
  // so a `count` reaches the crate's own `fold` (#162). Its source's by
  // `&mut` of what drops with `g`, `ends`, which it drops after; `own`, its
  // function, `$holding`'s, moved into its source's, as its `Map` is (#710).
  const fold = (init, g, unwind, ends, own) => {
    let folded = init;
    try {
      if (inner !== undefined) folded = $foldDropping(inner, folded, g, unwind);
      inner = undefined;
      if (ended) return folded;
      ended = true;
      const made = unwind === undefined ? f : (x, a) => $staged(f, x, undefined, a, unwind);
      const each = (a, x) => $foldDropping($stepsOf(made(x, a)), a, g, unwind);
      return typeof it.fold === "function" ? it.fold(folded, each, unwind, own) : $foldDropping(it, folded, each, unwind, own);
    } finally {
      if (ends) ends();
    }
  };
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
    if (!broke(folded)) {
      ended = true;
      it.return?.();
    }
    return folded;
  };
  // Its `count`, std's `FlattenCompat::count`: the own `count` of the one
  // it's in, then its source's fold of each one's own, the crate's where
  // one writes it, each given the items' drop (#736). What takes it closes
  // it as one throws, a panic, its source unless it's folded it, which
  // closes itself.
  const count = (drop) => {
    let counted = 0;
    if (inner !== undefined) {
      const front = inner;
      inner = undefined;
      counted = $count(front, drop);
    }
    if (ended) return counted;
    ended = true;
    const each = (n, x) => n + $count($stepsOf(f(x)), drop);
    return typeof it.fold === "function" ? it.fold(counted, each) : $foldDropping(it, counted, each);
  };
  $forward(flat, it, { fold, tryFold, count });
  // Its `count` its own, which what holds it passes on, a `skip()`'s, as
  // std's `Skip::count` asks its source's.
  flat.ownCount = true;
  // What std's `Flatten::last` takes, its inner iterator stepped part-way,
  // its source, unless it's ended, fused as std's is, and `f`, which make
  // the rest, let go as they're taken (`$lastDropping`, #645).
  flat.inners = () => {
    const front = inner;
    const source = ended ? undefined : it;
    inner = undefined;
    ended = true;
    return [front, source, f];
  };
  return flat;
}
