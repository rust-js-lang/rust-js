function $nextIf(it, f) {
  if (it.items === undefined) {
    const step = it.peek();
    return !step.done && f(step.value) ? it.next().value : undefined;
  }
  return it.at < it.end && f(it.items[it.at]) ? it.items[it.at++] : undefined;
}
