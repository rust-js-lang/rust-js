// `it.find(test)` of items with a destructor: the first that `test` holds
// of, each before it dropped by `drop` as it's passed, as std's `find` drops
// each it rejects (ADR 0098); boxed where it may look like `None` (ADR 0051).
// By its `tryFold`, as std's `find` is its `try_fold`: `it` is the `&mut`
// `find` takes, `$lent`'s, a `$Steps`, which has one; a `skip` asks its
// source `nth(n - 1)` (ADR 0932). The one `test` throws on, a panic, is
// dropped as it unwinds, as std's `find`'s closure holds it (#423).
function $findDropping(it, test, drop, boxed) {
  let item;
  const hit = it.tryFold(
    false,
    (_, x) => {
      let found;
      try {
        found = test(x);
      } catch (error) {
        drop?.(x);
        throw error;
      }
      if (found) {
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
