// `peek_mut()` of a `Peekable` of numbers or text: a handle on the item it
// gives next, whose write changes it there, as Rust's `&mut` to it does; of
// one at its end, `undefined` (ADR 0513).
function $peekCell(it) {
  if (it.items === undefined) {
    const step = it.peek();
    if (step.done) return undefined;
    return {
      get value() {
        return step.value;
      },
      set value(value) {
        step.value = value;
      },
    };
  }
  return it.at < it.end ? $mutAt(it.items, it.at) : undefined;
}
