// `it.find(test)` of items with a destructor: the first that `test` holds
// of, each before it dropped by `drop` as it's passed, as std's `find` drops
// each it rejects (ADR 0098); boxed where it may look like `None` (ADR 0051).
function $findDropping(it, test, drop, boxed) {
  for (const item of it) {
    if (test(item)) return boxed ? $some(item) : item;
    drop(item);
  }
  return undefined;
}
