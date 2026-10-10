function $nextBackSome(it) {
  return it.at < it.end ? $some(it.items[--it.end]) : undefined;
}
