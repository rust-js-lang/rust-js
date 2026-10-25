// `it.fold(init, f)` of a JS iterator whose `fold` is std's adapter's over
// the crate's own, as a dictionary's `fold` is called, of the value it's of
// (#214).
function $foldOf(it, init, f) {
  return it.fold(init, f);
}
