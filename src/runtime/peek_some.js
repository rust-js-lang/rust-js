function $peekSome(it) {
  if (it.items === undefined) {
    const step = it.peek();
    return step.done ? undefined : $some(step.value);
  }
  return it.at < it.end ? $some(it.items[it.at]) : undefined;
}
