function $peek(it) {
  if (it.items === undefined) {
    const step = it.peek();
    return step.done ? undefined : step.value;
  }
  return it.at < it.end ? it.items[it.at] : undefined;
}
