// A `OnceCell`'s or `OnceLock`'s `set(value)`: `Ok`, holding `Some(value)`,
// or `Err(value)` if it holds one already.
function $onceSet(cell, value) {
  if (cell.value !== undefined) return { TAG: "Err", _0: value };
  cell.value = $some(value);
  return { TAG: "Ok", _0: undefined };
}
