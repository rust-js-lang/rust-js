// `next_if(f)` of a `Peekable` (ADR 0557): its next item where `f` takes it,
// else kept as what it peeked at, as std's is, an end too.
function $nextIf(peekable, f) {
  const item = $next(peekable);
  if (item !== undefined && f(item)) return item;
  peekable.peeked = $some(item);
  return undefined;
}
