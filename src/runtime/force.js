// A `LazyCell`'s value, made by its `init` the first time.
// While it runs, and after it panics, `init` is `null`: std's poisoned.
function $force(lazy) {
  const init = lazy.init;
  if (init !== undefined) {
    if (init === null) throw new Error("LazyCell instance has previously been poisoned");
    lazy.init = null;
    lazy.value = init();
    lazy.init = undefined;
  }
  return lazy.value;
}
