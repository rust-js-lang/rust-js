// `it.nth(n)` of items with a destructor: its `n`th item, each before it
// dropped by `drop` as it's passed, as std's `nth` drops each it skips (ADR
// 0098); boxed where it may look like `None` (ADR 0051). Of one given the
// crate's own `nth`, `ownNth`, that one, which drops what it skips itself
// (#182).
function $nthDropping(it, n, drop, boxed) {
  // Or any `$Steps`' `nth`, std's default, `advance_by` by its `try_fold`,
  // each it passes given to `drop` (#294, #302).
  if (it.nth !== undefined) {
    // Given the items' drop, which a chain's side with none of its own,
    // a compiled struct's, advances by (#285).
    const step = it.nth(n, drop);
    return step.done ? undefined : boxed ? $some(step.value) : step.value;
  }
  for (const item of it) {
    if (n === 0) return boxed ? $some(item) : item;
    drop?.(item);
    n--;
  }
  return undefined;
}
