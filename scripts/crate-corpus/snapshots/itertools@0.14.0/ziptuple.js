// itertools@0.14.0's ziptuple.js: zipTuple2DoubleEndedIterator_nextBack, as scripts/crate-corpus.ts's `snapshotted` lists them.
// `BLESS=1 bun scripts/crate-corpus.ts --check itertools` writes it.

function zipTuple2DoubleEndedIterator_nextBack(
  zip,
  ADoubleEndedIterator,
  AExactSizeIterator,
  BDoubleEndedIterator,
  BExactSizeIterator,
  dropA,
  dropB,
  dropAItem,
  dropBItem,
) {
  let tmp$0$0$live = false;
  let tmp$1$0$live = false;
  const A = zip.t[0];
  const B = zip.t[1];
  const size = $unwrap(
    $min([AExactSizeIterator.len(A), BExactSizeIterator.len(B)]),
    "/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/itertools-0.14.0/src/ziptuple.rs:127:1",
  );
  if (AExactSizeIterator.len(A) !== size) {
    const end = (AExactSizeIterator.len(A) - size) >>> 0;
    for (let i = 0; i < end; i++) {
      const value = ADoubleEndedIterator.next_back(A);
      if (value != null) {
        dropAItem?.($someValue(value));
      }
    }
  }
  if (BExactSizeIterator.len(B) !== size) {
    const end = (BExactSizeIterator.len(B) - size) >>> 0;
    for (let i = 0; i < end; i++) {
      const value = BDoubleEndedIterator.next_back(B);
      if (value != null) {
        dropBItem?.($someValue(value));
      }
    }
  }
  const value = ADoubleEndedIterator.next_back(A);
  let tmp;
  let tmp$1;
  try {
    tmp = BDoubleEndedIterator.next_back(B);
    tmp$1$0$live = true;
    tmp$0$0$live = true;
    tmp$1 = [value, tmp];
  } catch (error) {
    $cleanup(() => {
      if (value != null) {
        dropAItem?.($someValue(value));
      }
    }, error);
    throw error;
  }
  let result;
  if (tmp$1[0] != null && tmp$1[1] != null) {
    tmp$0$0$live = false;
    const A = $someValue(tmp$1[0]);
    tmp$1$0$live = false;
    const B = $someValue(tmp$1[1]);
    result = [A, B];
  } else {
    result = undefined;
  }
  try {
    if (tmp$1[0] != null && tmp$0$0$live) {
      dropAItem?.($someValue(tmp$1[0]));
    }
  } catch (error) {
    $cleanup(() => {
      if (tmp$1[1] != null) {
        if (tmp$1$0$live) {
          dropBItem?.($someValue(tmp$1[1]));
        }
      }
    }, error);
    throw error;
  }
  if (tmp$1[1] != null && tmp$1$0$live) {
    dropBItem?.($someValue(tmp$1[1]));
  }
  return result;
}
