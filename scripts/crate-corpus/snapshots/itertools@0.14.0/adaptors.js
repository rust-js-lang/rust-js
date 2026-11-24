// itertools@0.14.0's adaptors.js: putBack, interleave, whileSomeIterator_next, as scripts/crate-corpus.ts's `snapshotted` lists them.
// `BLESS=1 bun scripts/crate-corpus.ts --check itertools` writes it.

/** Create an iterator where you can put back a single item */
export function putBack(iterable, IIntoIterator, dropIItem) {
  let iter;
  try {
    iter = $held(IIntoIterator.intoIter(iterable));
  } catch (error) {
    throw error;
  }
  return $brand("itertools#<hash>::adaptors::PutBack", { iter });
}

/**
 * Create an iterator that interleaves elements in `i` and `j`.
 *
 * [`IntoIterator`] enabled version of [`Itertools::interleave`](crate::Itertools::interleave).
 */
export function interleave(
  i,
  j,
  IIntoIterator,
  JIntoIterator,
  IIntoIterFused,
  JIntoIterFused,
  dropJ,
  dropIItem,
  dropIIntoIter,
  dropJIntoIter,
) {
  let fuse;
  try {
    fuse = iterator_fuse(
      $held(IIntoIterator.intoIter(i)),
      IIntoIterator.IntoIterIterator(),
      IIntoIterFused,
      dropIIntoIter,
      dropIItem,
    );
  } catch (error) {
    $cleanup(() => {
      dropJ?.(j);
    }, error);
    throw error;
  }
  let fuse$live = true;
  let tmp;
  try {
    tmp = iterator_fuse(
      $held(JIntoIterator.intoIter(j)),
      JIntoIterator.IntoIterIterator(),
      JIntoIterFused,
      dropJIntoIter,
      dropIItem,
    );
    fuse$live = false;
  } catch (error) {
    $cleanup(() => {
      if (fuse$live) {
        fuse.return();
      }
    }, error);
    throw error;
  }
  return $brand("itertools#<hash>::adaptors::Interleave", {
    i: fuse,
    j: tmp,
    nextComingFromJ: false,
  });
}

function whileSomeIterator_next(whileSome, IIterator, dropI, dropA) {
  const next = IIterator.next(whileSome.iter);
  let next$0$live = true;
  let result;
  if (next != null && $someValue(next) != null) {
    next$0$live = false;
    const elt = $someValue(next);
    result = elt;
  } else {
    result = undefined;
  }
  if (next != null) {
    if (next$0$live) {
      if ($someValue(next) != null) {
        dropA?.($someValue($someValue(next)));
      }
    }
  }
  return result;
}
