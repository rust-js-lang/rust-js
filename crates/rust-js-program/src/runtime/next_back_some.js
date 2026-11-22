function $nextBackSome(it) {
  const step = it.back();
  return step.done ? undefined : $some(step.value);
}
