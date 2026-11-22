// `it.count()`: its own `count` where it's a `$Steps`, a `Take`'s std's,
// which asks its source no `nth` (#213), each item given to `drop` where it
// has one; else each item counted as it comes. Not a `count` that's a
// field, an `$enumerate`'s, std's `Enumerate`'s, how many it's given (ADR
// 0988): its count is `countItems`, its source's (#493).
// Of one that may be a `Repeat`, or reach one's own `count`, where it was
// called, `raisedAt`, which that `count`'s panic names, as std's tracks
// its caller (#2324).
function $count(it, drop, raisedAt = undefined) {
  return $infinitelyAt(raisedAt, () => {
    if (typeof it.count === "function") return it.count(drop);
    if (it.countItems !== undefined) return it.countItems(drop);
    return it.reduce((n, x) => {
      drop?.(x);
      return n + 1;
    }, 0);
  });
}

// Where the `count` or `last` that may reach a `Repeat`'s own was called,
// held while `f` runs, which that one's panic names however many adapters
// pass the call on (`$repeating`'s `infinite`, #2324): under
// `$infiniteAtKey`, a slot of `globalThis` every copy of the runtime reads,
// as a library's copy may have made the `Repeat` (#2333's rule); the one
// it held before put back as `f` ends, however it ends, so a count inside
// another's closure, or a panic unwinding through one, leaves none stale.
function $infinitelyAt(raisedAt, f) {
  if (raisedAt === undefined) return f();
  const outer = globalThis[$infiniteAtKey];
  globalThis[$infiniteAtKey] = raisedAt;
  const made = $unwinds(f, () => {
    globalThis[$infiniteAtKey] = outer;
  });
  globalThis[$infiniteAtKey] = outer;
  return made;
}
