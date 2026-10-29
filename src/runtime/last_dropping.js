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
  if (it.sides !== undefined) {
    const [a, b] = it.sides();
    const aLast = a === undefined ? undefined : $lastDropping(a, drop, true);
    const bLast = $lastDropping(b, drop, true);
    if (bLast === undefined) return aLast === undefined ? undefined : some($someValue(aLast));
    if (aLast !== undefined) drop?.($someValue(aLast));
    return some($someValue(bLast));
  }
  if (it.ownLast) {
    const step = it.last();
    return step.done ? undefined : some(step.value);
  }
  let last;
  let seen = false;
  const keep = (item) => {
    if (seen) drop?.(last);
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
