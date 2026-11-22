// `max()` of `items`, numbers, `bool`s or `Ordering`s: std's
// `max_by(Ord::cmp)`, which steps them as each comes (#179).
function $max(items) {
  return $maxBy(items, $cmp);
}
