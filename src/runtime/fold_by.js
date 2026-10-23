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

// A `Fuse`'s fold, std's: its source's, while it has one, which it lets go
// of, dropped by `drop`, as its `next` does at its end.
function $fuseFold(fuse, init, f, iterator, drop) {
  if (fuse.iter == null) return init;
  const iter = $someValue(fuse.iter);
  fuse.iter = undefined;
  return $foldBy(iter, init, f, iterator, drop);
}
