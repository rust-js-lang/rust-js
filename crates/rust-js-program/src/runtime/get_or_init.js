// A `OnceCell`'s `get_or_init(f)`: what it holds, made by
// `f` the first time, which may not set it itself.
function $getOrInit(cell, f) {
  if (cell.value === undefined) {
    const value = f();
    if (cell.value !== undefined) throw $panic("reentrant init", $stdSites.onceCellReentrant[0]);
    cell.value = $some(value);
  }
  return $someValue(cell.value);
}
