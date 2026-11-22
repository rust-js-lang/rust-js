// The crate's own `IntoIterator`, `&Board`'s, given where std's code or
// generic code takes any `IntoIterator`, `v.extend(&board)`'s (#1940, ADR
// 1171): an iterable whose iterator is the one its own `into_iter` makes,
// `make()`, made as what takes it asks for it, where Rust calls
// `into_iter`: as a zip or a chain is made, as an extend starts, as generic
// code calls `into_iter`.
function $ownIntoIter(make) {
  return { [Symbol.iterator]: make };
}
