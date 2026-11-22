// `next_if(f)` of a `Peekable` (ADR 0561): its next item where `f` takes it,
// else kept as what it peeked at, as std's is, an end too. Where `f`
// throws, a panic, the item it was asked of dropped by `drop` as it
// unwinds, as std's `next_if` holds it in a local then (#1648).
function $nextIf(peekable, f, drop) {
  const item = $next(peekable);
  if (item !== undefined && $unwinds(() => f(item), drop && (() => drop(item)))) return item;
  peekable.peeked = $some(item);
  return undefined;
}
// Of items that may look like `None`, `Option`s: each boxed (ADR 0051),
// `f` given it unboxed, and dropped so too.
function $nextIfSome(peekable, f, drop) {
  const item = $nextSome(peekable);
  if (item !== undefined && $unwinds(() => f($someValue(item)), drop && (() => drop($someValue(item))))) return item;
  peekable.peeked = $some(item);
  return undefined;
}
