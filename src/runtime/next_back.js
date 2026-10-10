function $nextBack(it) {
  return it.at < it.end ? it.items[--it.end] : undefined;
}
