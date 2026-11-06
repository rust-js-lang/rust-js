// `a.zip(b)`, lazy: a pair of each's next, till either ends; an item of
// `a`'s stepped where `b`'s run out, or where `b`'s step throws, given to
// `dropA`, as Rust drops it;
// both closed as it's closed, which drops what they hold (ADR 0465). A
// step after its end steps each on, as Rust's `Zip`, which isn't fused,
// does. Each side is held, as std's `Zip` owns it: one that ends doesn't
// close itself there, before `dropA` drops the other's item, but as the
// zip is closed, `a` then `b`.
function $lazyZip(a, b, dropA) {
  const left = $stepsOf(a);
  const right = $stepsOf(b);
  left.hold?.();
  right.hold?.();
  // `a` then `b`, the second though the first's drop throws (#743).
  const close = () => $closeEach(left, right);
  const zipped = new $Steps(() => {
    const item = left.next();
    if (item.done) return item;
    // Where `b`'s step throws, a panic, `a`'s item dropped as it unwinds,
    // as std's `Zip::next` holds it (#695).
    const other = $unwinds(
      () => right.next(),
      () => dropA?.(item.value),
    );
    if (other.done) {
      dropA?.(item.value);
      return other;
    }
    return { done: false, value: [item.value, other.value] };
  }, close);
  zipped.from = [left, right];
  // Its `size_hint()`, std's `Zip`'s of its sides', each what its JS
  // value tells, as a generic iterator's is (#400).
  zipped.sizeHint = () => $zipHint($sizeHint(left), $sizeHint(right));
  return zipped;
}

// A zip's `size_hint()` of its sides', std's `Zip`'s: the fewer of their
// lower bounds, and of their upper, the one either has, or none (#400).
function $zipHint([aLow, aHigh], [bLow, bHigh]) {
  const high = aHigh === undefined ? bHigh : bHigh === undefined ? aHigh : Math.min(aHigh, bHigh);
  return [Math.min(aLow, bLow), high];
}

// `a.zip(b)` std reads by index, its sides `TrustedRandomAccess` and a
// step of `a` running code, a `map`'s closure: a pair of each's next while
// both have one, `a` never stepped once `b`, which knows its length, has
// none, as std's zip pairs them up to the shorter's length (ADR 0973).
function $trustedZip(a, b) {
  const left = $stepsOf(a);
  const zipped = new $Steps(
    () => {
      if (b.len() === 0) return { done: true, value: undefined };
      const item = left.next();
      if (item.done) return item;
      return { done: false, value: [item.value, b.next().value] };
    },
    () => $closeEach(left, b),
  );
  zipped.from = [left, b];
  return zipped;
}

// `__iterator_get_unchecked(i)` of `it`, which std's compiled `Zip` reads
// its sides by (ADR 0975): std's compiled adapter's own reader, its
// dictionary's; an array's `$iter`'s item `i` past where it is, unstepped;
// a range's start plus `i`, of a 64-bit one a BigInt's.
function $getUnchecked(it, i, dictionary) {
  if (dictionary?.getUnchecked !== undefined) return dictionary.getUnchecked(it, i);
  if (it.items !== undefined) return it.items[it.at + i];
  return typeof it.start === "bigint" ? it.start + BigInt(i) : it.start + i;
}

// A zip of two iterators made knowing their backs and lengths, `zipped`
// of `left` and `right`, as std's `Zip` is double-ended and exact-sized
// where both its sides are: its length the shorter's, and its back std's
// `next_back`, the longer trimmed from its back to the shorter's length,
// each it trims dropped, `dropA` or `dropB`, then each's back paired;
// where `right`'s step throws, a panic, `left`'s item dropped as the pair
// std makes unwinds (#740). Each side's length asked once a step, `a`'s
// then `b`'s, as std's `next_back` asks `a.len()` and `b.len()` before it
// trims: the crate's own `size_hint` runs as often as natively (#824).
function $zipEndsOf(zipped, left, right, dropA, dropB) {
  zipped.len = () => Math.min(left.len(), right.len());
  zipped.back = () => {
    const aLen = left.len();
    const bLen = right.len();
    for (let n = aLen - bLen; n > 0; n--) {
      const x = left.back();
      if (!x.done) dropA?.(x.value);
    }
    for (let n = bLen - aLen; n > 0; n--) {
      const y = right.back();
      if (!y.done) dropB?.(y.value);
    }
    const x = left.back();
    const y = $unwinds(
      () => right.back(),
      () => x.done || dropA?.(x.value),
    );
    if (x.done || y.done) return { done: true, value: undefined };
    return { done: false, value: [x.value, y.value] };
  };
  return zipped;
}

// `a.zip(b)`, `$lazyZip`'s, of sides made knowing their backs and lengths.
// One std folds by `SpecFold`'s `TrustedLen` impl, `toShorter`, of sides
// both `TrustedLen` but not both `TrustedRandomAccess` (ADR 0997): its
// `fold` takes as many pairs as the shorter side has, never stepping `a`
// once more as its `next` does; what's left of each dropped as it's
// closed, as std drops the zip (#812).
function $zipEnds(a, b, dropA, dropB, toShorter) {
  const zipped = $lazyZip(a, b, dropA);
  const left = zipped.from[0];
  const right = zipped.from[1];
  if (toShorter) {
    const fold = (init, f, unwind, ends) => {
      let pairs = Math.min(left.len(), right.len());
      const next = zipped.next;
      zipped.next = () => (pairs-- > 0 ? next() : { done: true, value: undefined });
      try {
        return $Steps.prototype.fold.call(zipped, init, f, unwind, ends);
      } finally {
        zipped.next = next;
      }
    };
    $forward(zipped, zipped, { fold });
  }
  return $zipEndsOf(zipped, left, right, dropA, dropB);
}

// `a.zip(b)` std reads by index, `$trustedZip`'s, of sides made knowing
// their backs and lengths.
function $trustedZipEnds(a, b, dropA, dropB) {
  const zipped = $trustedZip(a, b);
  return $zipEndsOf(zipped, zipped.from[0], zipped.from[1], dropA, dropB);
}
