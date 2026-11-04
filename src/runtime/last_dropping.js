// `it.last()` of items with a destructor: its last item, each before it
// dropped by `drop` as the next comes, as std's `last` drops each it passes
// (ADR 0098), `boxed` where an item could look like `None`, `Some` of it
// that is `$some`'s (ADR 0051), then closed, as `last(self)` drops what
// it has left at its end (#242). Of one given the crate's own `last`,
// `ownLast`, that one, which drops what it passes itself (#182).
function $lastDropping(it, drop, boxed) {
  const some = (item) => (boxed ? $some(item) : item);
  // Std's `Chain::last`: each side's own `last`, `a`'s then `b`'s, each
  // dropping what it passes; `b`'s where it has one, `a`'s dropped then,
  // as `b_last.or(a_last)` drops it (#232).
  // Std's `Skip::last`: its source's, after what it skips (#232).
  if (it.skipped !== undefined) {
    const source = it.skipped();
    if (source !== undefined) return $lastDropping(source, drop, boxed);
    it.return?.();
    return undefined;
  }
  // Where `a`'s throws, a panic, `b` closed, as `last(self)` drops what it
  // holds as it unwinds; where `b`'s does, `a`'s last dropped (#647).
  if (it.sides !== undefined) {
    const [a, b] = it.sides();
    let aLast;
    try {
      aLast = a === undefined ? undefined : $lastDropping(a, drop, true);
    } catch (error) {
      b.return?.();
      throw error;
    }
    let bLast;
    try {
      bLast = $lastDropping(b, drop, true);
    } catch (error) {
      if (aLast !== undefined) drop?.($someValue(aLast));
      throw error;
    }
    if (bLast === undefined) return aLast === undefined ? undefined : some($someValue(aLast));
    if (aLast !== undefined) drop?.($someValue(aLast));
    return some($someValue(bLast));
  }
  // Std's `Flatten::last`: each inner iterator's own `last`, the one it's
  // in first, then each its source makes, by its fold; each dropping what
  // it passes, then the one kept before where it has one, as
  // `iter.last().or(last)` drops it (#645). Where one throws, a panic, or
  // `f` does, the one kept dropped, then what's left of the source, as the
  // fold's closure drops the accumulator it holds as it unwinds.
  if (it.inners !== undefined) {
    const [front, source, f] = it.inners();
    let kept;
    const unwind = () => {
      const dropped = kept;
      kept = undefined;
      if (dropped !== undefined) drop?.($someValue(dropped));
    };
    const take = (make) => {
      let last;
      try {
        last = $lastDropping(make(), drop, true);
      } catch (error) {
        unwind();
        throw error;
      }
      if (last === undefined) return;
      if (kept !== undefined) drop?.($someValue(kept));
      kept = last;
    };
    if (front !== undefined) {
      try {
        take(() => front);
      } catch (error) {
        source?.return?.();
        throw error;
      }
    }
    if (source !== undefined && Object.hasOwn(source, "fold")) {
      source.fold(undefined, (_, item) => take(() => $stepsOf(f(item))), drop && unwind);
    } else if (source !== undefined) {
      try {
        for (let step = source.next(); !step.done; step = source.next()) take(() => $stepsOf(f(step.value)));
      } catch (error) {
        unwind();
        source.return?.();
        throw error;
      }
      source.return?.();
    }
    return kept === undefined ? undefined : some($someValue(kept));
  }
  // Given `drop`, which a `Vec`'s `IntoIter`'s own, `$slicing`'s, drops
  // what it has left by (#648).
  if (it.ownLast) {
    const step = it.last(drop);
    return step.done ? undefined : some(step.value);
  }
  let last;
  let seen = false;
  // The one kept is given up before it's dropped as the next replaces it:
  // a drop that panics is its own, not dropped again as the panic unwinds,
  // as std's `last` drops its `Option` once (#648).
  const keep = (item) => {
    if (seen) {
      seen = false;
      drop?.(last);
    }
    last = item;
    seen = true;
  };
  // Std's default `last`, its `fold`: an adapter's own, a `take_while`'s of
  // a `skip`'s `try_fold`, which asks `nth(n - 1)` where its `next` asks
  // `nth(n)` (#299). A `$Steps`' `fold` closes it at its end, and where a
  // stage throws, a panic, before the one kept is dropped here (#423).
  try {
    // The one kept dropped where a stage throws, as std's fold closure
    // drops the accumulator it holds, before what's left (#423).
    const unwind = () => {
      if (seen) {
        seen = false;
        drop?.(last);
      }
    };
    if (Object.hasOwn(it, "fold")) {
      it.fold(undefined, (_, item) => keep(item), drop && unwind);
      return seen ? some(last) : undefined;
    }
    for (const item of it) keep(item);
  } catch (error) {
    // The one kept, then what's left, as std's `fold` drops its own as it
    // unwinds, then what it took.
    try {
      if (seen) drop?.(last);
    } finally {
      it.return?.();
    }
    throw error;
  }
  it.return?.();
  return seen ? some(last) : undefined;
}
