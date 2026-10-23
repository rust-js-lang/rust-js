
function $fromFn(f, boxed = false) {
  return new $Steps(() => {
    const item = f();
    if (item == null) {
      return { done: true, value: undefined };
    }
    return { done: false, value: boxed ? $someValue(item) : item };
  });
}
