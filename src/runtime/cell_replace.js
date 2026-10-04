// A `Cell`'s or `RefCell`'s `replace(value)`, `take()` and `replace_with(f)`:
// what it held, with `value` in its place.
function $cellReplace(cell, value) {
  const previous = cell.value;
  cell.value = value;
  return previous;
}
