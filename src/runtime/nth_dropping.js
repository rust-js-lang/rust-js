// `it.nth(n)` of items with a destructor: its `n`th item, each before it
// dropped by `drop` as it's passed, as std's `nth` drops each it skips (ADR
// 0098); boxed where it may look like `None` (ADR 0051).
function $nthDropping(it, n, drop, boxed) {
  for (const item of it) {
    if (n === 0) return boxed ? $some(item) : item;
    drop(item);
    n--;
  }
  return undefined;
}
