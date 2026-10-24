// `it.nth(k)` of an iterator whose `nth` is its own, the crate's or std's
// adapter's (#182): its item, or `undefined` for `None`, as `$next`'s.
function $nth(it, k) {
  const step = it.nth(k);
  return step.done ? undefined : step.value;
}

// `it.nth(k)` of items that look like `None`: its `Some` boxed (ADR 0051).
function $nthSome(it, k) {
  const step = it.nth(k);
  return step.done ? undefined : $some(step.value);
}

// `it.last()` of an iterator whose `last` is the crate's own, or std's
// adapter's passing it on (#182): its item, or `undefined` for `None`.
function $last(it) {
  const step = it.last();
  return step.done ? undefined : step.value;
}

// `it.last()` of items that look like `None`: its `Some` boxed (ADR 0051).
function $lastSome(it) {
  const step = it.last();
  return step.done ? undefined : $some(step.value);
}
