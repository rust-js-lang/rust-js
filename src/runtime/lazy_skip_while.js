
// `it.skip_while(skip)`: its items from the first `skip` doesn't hold of;
// a `None` of its source while it skips ends what's stepped now, as Rust's
// `find` gives it.
function $lazySkipWhile(items, skip) {
  const source = items[Symbol.iterator]();
  let skipping = true;
  return new $Steps(
    () => {
      for (;;) {
        const step = source.next();
        if (step.done || !skipping || !skip(step.value)) {
          if (!step.done) skipping = false;
          return step;
        }
      }
    },
    () => source.return?.(),
  );
}
