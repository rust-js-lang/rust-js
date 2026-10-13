function $rest(it) {
  // What's left of a stepped iterator.
  const rest = it.items.slice(it.at);
  it.at = it.end;
  return rest;
}
