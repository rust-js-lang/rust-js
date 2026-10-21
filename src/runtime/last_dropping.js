// `it.last()` of items with a destructor: its last item, each before it
// dropped by `drop` as the next comes, as std's `last` drops each it passes
// (ADR 0098).
function $lastDropping(it, drop) {
  let last;
  let seen = false;
  for (const item of it) {
    if (seen) drop?.(last);
    last = item;
    seen = true;
  }
  return last;
}
