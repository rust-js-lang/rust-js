// `next_if(f)` of a `Peekable` (ADR 0561): its next item where `f` takes it,
// else kept as what it peeked at, as std's is, an end too.
function $nextIf(peekable, f) {
  const item = $next(peekable);
  if (item !== undefined && f(item)) return item;
  peekable.peeked = $some(item);
  return undefined;
}
// Of items that may look like `None`, `Option`s: each boxed (ADR 0051),
// `f` given it unboxed.
function $nextIfSome(peekable, f) {
  const item = $nextSome(peekable);
  if (item !== undefined && f($someValue(item))) return item;
  peekable.peeked = $some(item);
  return undefined;
}
