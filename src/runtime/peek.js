// `peek()` of a `Peekable`, std's `{ iter, peeked }` (ADR 0557), whose body
// reaches memory: what it peeked at, or what it steps next, its own `next`'s
// (`$stdIterator`), kept as `Some` of it, as Rust's `get_or_insert_with`
// keeps it; the item, or `undefined` at the end.
function $peek(peekable) {
  if (peekable.peeked === undefined) peekable.peeked = $some($next(peekable));
  return $someValue(peekable.peeked);
}
// Of items that may look like `None`, `Option`s: each boxed (ADR 0051).
function $peekSome(peekable) {
  if (peekable.peeked === undefined) peekable.peeked = $some($nextSome(peekable));
  return $someValue(peekable.peeked);
}
// Its size hint, Rust's: what it steps', and what it peeked at, an end none.
function $peekableHint(peekable) {
  if (peekable.peeked !== undefined && $someValue(peekable.peeked) === undefined) return [0, 0];
  const more = peekable.peeked === undefined ? 0 : 1;
  const [low, high] = $sizeHint(peekable.iter);
  return [low + more, high === undefined ? undefined : high + more];
}
