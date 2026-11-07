// `v.dedup_by(same)` of items a `&mut` to is a cell, a number's, a text's,
// a `Box`'s or an `Rc`'s: `$dedupBy`'s, each given `same` as `{ value }`
// (#906).
function $dedupByCells(v, same, drop) {
  $dedupBy(v, same, drop, true);
}
