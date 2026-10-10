
function $peek(it) {
  return it.at < it.end ? it.items[it.at] : undefined;
}
