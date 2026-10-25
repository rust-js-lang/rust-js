// `fold` of `it`, by its `Iterator` dictionary, `iterator` (ADR 0411): the
// crate's own, where its impl writes one (#162), which takes `it` and drops
// it; a JS iterator's, its `reduce`, which a `$Steps` folds by its source's;
// else each `next`, its `Some` unboxed (ADR 0051). Std's `fold` takes `it`:
// stepped to its end here, it's dropped by `drop` after, of one with a
// destructor.
function $foldBy(it, init, f, iterator, drop) {
  if (iterator?.fold !== undefined) return iterator.fold(it, init, f);
  let folded = init;
  if (iterator === undefined || iterator.next === $next || iterator.next === $nextSome) {
    folded = it.reduce((folded, item) => f(folded, item), init);
  } else {
    for (let item = iterator.next(it); item != null; item = iterator.next(it)) folded = f(folded, $someValue(item));
  }
  drop?.(it);
  return folded;
}

// A `Peekable`'s fold, std's: what it peeked at, unless that was its end,
// then its source's. Its source is dropped with it, as a struct's fields
// are, where it's consumed; but the crate's own fold took it and dropped
// it, which `ownFold` says, as `$iterator`'s does.
function $peekableFold(peekable, init, f, iterator) {
  if (iterator?.fold !== undefined) peekable.ownFold = true;
  let folded = init;
  if (peekable.peeked !== undefined) {
    const peeked = $someValue(peekable.peeked);
    peekable.peeked = undefined;
    if (peeked == null) return init;
    folded = f(folded, $someValue(peeked));
  }
  return $foldBy(peekable.iter, folded, f, iterator);
}

// A `Map`'s fold, std's: its source's, each item given its `f`, by the
// source's dictionary, which has that impl's own `fold` (#264).
function $mapFold(map, init, g, iterator) {
  const f = map.f;
  return $foldBy(map.iter, init, (acc, x) => g(acc, f(x)), iterator);
}

// An `Inspect`'s fold, std's: its source's, `f` given each item first.
function $inspectFold(inspect, init, g, iterator) {
  const f = inspect.f;
  return $foldBy(
    inspect.iter,
    init,
    (acc, x) => {
      f(x);
      return g(acc, x);
    },
    iterator,
  );
}

// A `Fuse`'s fold, std's: its source's, while it has one, which it lets go
// of, dropped by `drop`, as its `next` does at its end.
function $fuseFold(fuse, init, f, iterator, drop) {
  if (fuse.iter == null) return init;
  const iter = $someValue(fuse.iter);
  fuse.iter = undefined;
  return $foldBy(iter, init, f, iterator, drop);
}

// A `Fuse`'s `rfold`, std's: its source's, while it has one, by its
// dictionaries, `iterator` and `back`: the crate's own `rfold` where its impl
// writes one, which takes the source and drops it, else each `next_back`,
// the source dropped by `drop` after (#224).
function $fuseRfold(fuse, init, f, iterator, back, drop) {
  if (fuse.iter == null) return init;
  const iter = $someValue(fuse.iter);
  fuse.iter = undefined;
  if (back?.rfold !== undefined) return back.rfold(iter, init, f);
  const folded = $foldBack($given(iter, iterator, true, back), init, f);
  drop?.(iter);
  return folded;
}

// `try_fold` of `it`, by its `Iterator` dictionary, `iterator`, as
// `$foldBy`'s fold (#214): a JS iterator's own `tryFold`, which a `$Steps`
// passes to its source's as std's adapters do; else each `next`, its
// `Some` unboxed, until `broke` says stop.
function $tryFoldBy(it, init, f, broke, iterator) {
  let folded = init;
  if (iterator === undefined || iterator.next === $next || iterator.next === $nextSome) {
    if (it.tryFold !== undefined) return it.tryFold(init, f, broke);
    for (let step = it.next(); !step.done; step = it.next()) {
      folded = f(folded, step.value);
      if (broke(folded)) return folded;
    }
    return folded;
  }
  for (let item = iterator.next(it); item != null; item = iterator.next(it)) {
    folded = f(folded, $someValue(item));
    if (broke(folded)) return folded;
  }
  return folded;
}

// A `Peekable`'s `try_fold`, std's: what it peeked at, unless that was its
// end, then its source's (#214).
function $peekableTryFold(peekable, init, f, broke, iterator) {
  let folded = init;
  if (peekable.peeked !== undefined) {
    const peeked = $someValue(peekable.peeked);
    peekable.peeked = undefined;
    if (peeked == null) return init;
    folded = f(folded, $someValue(peeked));
    if (broke(folded)) return folded;
  }
  return $tryFoldBy(peekable.iter, folded, f, broke, iterator);
}

// A `Fuse`'s `try_fold`, std's: its source's, while it has one, which it
// lets go of, dropped by `drop`, once that's run to its end (#214); of a
// `FusedIterator`, `fused`, kept, as std's specialization keeps it (#244).
function $fuseTryFold(fuse, init, f, broke, iterator, drop, fused = false) {
  if (fuse.iter == null) return init;
  const iter = $someValue(fuse.iter);
  const folded = $tryFoldBy(iter, init, f, broke, iterator);
  if (!fused && !broke(folded)) {
    fuse.iter = undefined;
    drop?.(iter);
  }
  return folded;
}
