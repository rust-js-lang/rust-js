// `it.count()`: its own `count` where it's a `$Steps`, a `Take`'s std's,
// which asks its source no `nth` (#213), each item given to `drop` where it
// has one; else each item counted as it comes. Not a `count` that's a
// field, an `$enumerate`'s, std's `Enumerate`'s, how many it's given (ADR
// 0988).
function $count(it, drop) {
  if (typeof it.count === "function") return it.count(drop);
  return it.reduce((n, x) => {
    drop?.(x);
    return n + 1;
  }, 0);
}
