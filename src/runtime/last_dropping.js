// `it.last()` of items with a destructor: its last item, each before it
// dropped by `drop` as the next comes, as std's `last` drops each it passes
// (ADR 0098). Of one given the crate's own `last`, `ownLast`, that one,
// which drops what it passes itself (#182).
function $lastDropping(it, drop) {
  if (it.ownLast) {
    const step = it.last();
    return step.done ? undefined : step.value;
  }
  let last;
  let seen = false;
  for (const item of it) {
    if (seen) drop?.(last);
    last = item;
    seen = true;
  }
  return last;
}
