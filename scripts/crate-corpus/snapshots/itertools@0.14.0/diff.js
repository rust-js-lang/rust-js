// itertools@0.14.0's diff.js: diffClone_clone, as scripts/crate-corpus.ts's `snapshotted` lists them.
// `BLESS=1 bun scripts/crate-corpus.ts --check itertools` writes it.

function diffClone_clone(diff, PutBackClone, PutBackClone$1, dropI, dropJ, dropIItem, dropJItem) {
  let value$live = false;
  if (diff.TAG !== "FirstMismatch") {
    if (diff.TAG === "Shorter") {
      return { TAG: "Shorter", _0: diff._0, _1: PutBackClone.clone(diff._1) };
    }
    return { TAG: "Longer", _0: diff._0, _1: PutBackClone$1.clone(diff._1) };
  }
  const value = PutBackClone.clone(diff._1);
  value$live = true;
  let tmp;
  let result;
  try {
    tmp = PutBackClone$1.clone(diff._2);
    value$live = false;
    result = { TAG: "FirstMismatch", _0: diff._0, _1: value, _2: tmp };
  } catch (error) {
    $cleanup(() => {
      if (value$live) {
        try {
          if (value.top != null) {
            dropIItem?.($someValue(value.top));
          }
        } catch (error) {
          $cleanup(() => {
            dropI?.(value.iter);
          }, error);
          throw error;
        }
        dropI?.(value.iter);
      }
    }, error);
    throw error;
  }
  return result;
}
