// A `OnceLock`'s `get_or_init(f)`: what it holds, made by `f` the first
// time. An `f` that sets it itself deadlocks in Rust, which JS can't do.
function $getOrInitLock(cell, f) {
  if (cell.value === undefined) {
    const value = f();
    if (cell.value !== undefined) throw new Error("rust-js does not support a `OnceLock` whose init sets it, which deadlocks in Rust");
    cell.value = $some(value);
  }
  return $someValue(cell.value);
}
