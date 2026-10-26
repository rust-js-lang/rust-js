// `it.find(test)` of items with a destructor: the first that `test` holds
// of, each before it dropped by `drop` as it's passed, as std's `find` drops
// each it rejects (ADR 0098); boxed where it may look like `None` (ADR 0051).
// By its `tryFold`, as std's `find` is its `try_fold`, where it has one, a
// `$Steps`'s or std's struct's: a `skip` asks its source `nth(n - 1)` (ADR
// 0932).
function $findDropping(it, test, drop, boxed) {
  if (it.tryFold !== undefined) {
    let item;
    const hit = it.tryFold(
      false,
      (_, x) => {
        if (test(x)) {
          item = x;
          return true;
        }
        drop?.(x);
        return false;
      },
      (hit) => hit,
    );
    if (!hit) return undefined;
    it.return?.();
    return boxed ? $some(item) : item;
  }
  for (const item of it) {
    if (test(item)) return boxed ? $some(item) : item;
    drop?.(item);
  }
  return undefined;
}
