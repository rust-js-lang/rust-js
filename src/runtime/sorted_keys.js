function $sortedKeys(set, cmp) {
  return $sort(Array.from(set), cmp);
}
