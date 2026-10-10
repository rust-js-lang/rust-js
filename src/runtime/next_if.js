
function $nextIf(it, f) {
  return it.at < it.end && f(it.items[it.at]) ? it.items[it.at++] : undefined;
}
