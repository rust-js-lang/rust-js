// A `LazyLock`'s value, made by its `init` the first time. An init that
// uses its own `LazyLock` deadlocks in Rust, which JS can't do.
function $forceLock(lazy) {
  const init = lazy.init;
  if (init !== undefined) {
    if (init === null) throw new Error("rust-js does not support a `LazyLock` whose init uses it, which deadlocks in Rust");
    lazy.init = null;
    lazy.value = init();
    lazy.init = undefined;
  }
  return lazy.value;
}
