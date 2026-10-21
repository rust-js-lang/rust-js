// `peek_mut()` of a `Peekable` of numbers or text: a handle on the item it
// gives next, whose write changes it there, as Rust's `&mut` to it does; of
// one at its end, `undefined` (ADR 0513).
function $peekCell(peekable, next) {
  if ($peek(peekable, next) === undefined) return undefined;
  return {
    get value() {
      return $someValue(peekable.peeked);
    },
    set value(value) {
      peekable.peeked = $some(value);
    },
  };
}
