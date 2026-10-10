
function $rest(it) {
  const rest = it.items.slice(it.at, it.end);
  it.at = it.end;
  return rest;
}
