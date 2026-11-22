// A `LazyLock`'s value, made by its `init` the first time. An init that
// uses its own `LazyLock` deadlocks in Rust, which JS can't do. One that
// panics poisons it as it unwinds, as std's `Once` guard's drop does: each
// later use panics (#1438).
function $forceLock(lazy) {
  const init = lazy.init;
  if (init !== undefined) {
    if (init === null) throw $unsupported("rust-js does not support a `LazyLock` whose init uses it, which deadlocks in Rust");
    lazy.init = null;
    lazy.value = $unwinds(init, () => {
      lazy.init = $lazyLockPoisoned;
    });
    lazy.init = undefined;
  }
  return lazy.value;
}

function $lazyLockPoisoned() {
  throw new Error("LazyLock instance has previously been poisoned");
}
