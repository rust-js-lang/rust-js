// `iter::once_with(f)`: `f` called when it is first stepped, its value the one item.
function* $onceWith(f) {
  yield f();
}
