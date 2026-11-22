// `it.nth(n)` of items with a destructor, `it` lent (`$lent`), so a
// `$Steps`: its `n`th item, each before it given to `drop` as it's passed,
// as std's `nth` drops each it skips (ADR 0098), by its `nth`, std's
// default, `advance_by` by its `try_fold` (#294, #302), or the crate's own,
// `ownNth`, which drops what it skips itself (#182); boxed where it may
// look like `None` (ADR 0051).
function $nthDropping(it, n, drop, boxed) {
  // Given the items' drop, which a chain's side with none of its own,
  // a compiled struct's, advances by (#285).
  const step = it.nth(n, drop);
  return step.done ? undefined : boxed ? $some(step.value) : step.value;
}
