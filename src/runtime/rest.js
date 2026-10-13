function $rest(it) {
  // A JS iterator, a generic one's (ADR 0061): what's left of it.
  if (it.items === undefined) {
    return Array.from(it);
  }
  const rest = it.items.slice(it.at, it.end);
  it.at = it.end;
  return rest;
}
