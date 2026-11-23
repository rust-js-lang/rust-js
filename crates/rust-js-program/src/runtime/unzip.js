// `unzip()` of `pairs`, an array or an iterator of pairs: each side's
// items, put in as each pair comes, by std's `fold`, which an iterator
// rust-js makes routes through the crate's own (#162). Where a step throws,
// a panic, what's left of `pairs` closed by its fold, then what each side
// collected dropped, `dropA`'s then `dropB`'s, as std's `(A, B)` drops its
// fields as it unwinds (#1150).
function $unzip(pairs, dropA, dropB, sizeA, sizeB, trusted, stdLower) {
  const a = [];
  const b = [];
  // std's `default_extend` of the pair of sides: the hint's lower bound
  // asked first, each side given room for it, `extend_reserve`, a `Vec`'s
  // `reserve` where its capacity is kept, of items of `sizeA` and `sizeB`
  // bytes, then each pair's own pushed, as a `Vec` grows by `push`
  // (#2405). Of a `TrustedLen`, `trusted`, whose lower bound is how many
  // it gives and whose hint runs nothing of the crate's, room for that
  // many, `$reservedFor` once they're in, whatever JS's form of it hints.
  // Of another, its hint asked, of an array its length; where std's lower
  // bound is always 0, a `filter`'s, which an array JS filtered at once
  // counts otherwise, `stdLower`, std's.
  const kept = (size) => size !== undefined && !trusted;
  if (!trusted) {
    const [asked] = Array.isArray(pairs) ? [pairs.length] : $sizeHint(pairs);
    const lower = stdLower ?? asked;
    if (lower > 0) {
      if (kept(sizeA)) $reserve(a, lower, sizeA);
      if (kept(sizeB)) $reserve(b, lower, sizeB);
    }
  }
  const put = ([x, y]) => {
    const len = a.length;
    a.push(x);
    if (kept(sizeA)) $grew(a, sizeA, len);
    b.push(y);
    if (kept(sizeB)) $grew(b, sizeB, len);
  };
  $unwinds(
    () => (typeof pairs.fold === "function" ? pairs.fold(undefined, (_, pair) => put(pair)) : $eachOf(pairs, put)),
    dropA && (() => $dropItems(a, dropA)),
    dropB && (() => $dropItems(b, dropB)),
  );
  if (trusted) {
    if (sizeA !== undefined) $reservedFor(a, sizeA);
    if (sizeB !== undefined) $reservedFor(b, sizeB);
  }
  return [a, b];
}
