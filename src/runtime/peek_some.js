function $peekSome(it) {
  return it.at < it.end ? $some(it.items[it.at]) : undefined;
}
