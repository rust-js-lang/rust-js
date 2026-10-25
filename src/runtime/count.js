// `it.count()`: its own `count` where it's a `$Steps`, a `Take`'s std's,
// which asks its source no `nth` (#213), each item given to `drop` where it
// has one; else each item counted as it comes.
function $count(it, drop) {
  if (it.count !== undefined) return it.count(drop);
  return it.reduce((n, x) => {
    drop?.(x);
    return n + 1;
  }, 0);
}
