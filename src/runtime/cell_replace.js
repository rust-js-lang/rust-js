// A `Cell`'s or `RefCell`'s `replace(value)`, `take()` and `replace_with(f)`:
// what it held, with `value` in its place.
function $cellReplace(cell, value) {
  const previous = cell.value;
  cell.value = value;
  return previous;
}

// `a.swap(&b)` of two cells: each the other's value, a copy apart of it.
// Either may be an in-place cell, `Cell::from_mut`'s of an object, whose
// write changes the object its `value` gave, so each is moved out apart
// before either is written (#527).
function $cellSwap(a, b) {
  [a.value, b.value] = [$apart(b.value), $apart(a.value)];
}
