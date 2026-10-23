// `min()` of `items`, numbers, `bool`s or `Ordering`s: std's
// `min_by(Ord::cmp)`, which steps them as each comes (#179).
function $min(items) {
  return $minBy(items, $cmp);
}
