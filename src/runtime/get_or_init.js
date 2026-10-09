// A `OnceCell`'s or `OnceLock`'s `get_or_init(f)`: what it holds, made by
// `f` the first time, which may not set it itself.
function $getOrInit(cell, f) {
  if (cell.value === undefined) {
    const value = f();
    if (cell.value !== undefined) throw new Error("reentrant init");
    cell.value = $some(value);
  }
  return $someValue(cell.value);
}
