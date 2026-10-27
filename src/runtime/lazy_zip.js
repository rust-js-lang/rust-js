// `a.zip(b)`, lazy: a pair of each's next, till either ends; an item of
// `a`'s stepped where `b`'s run out given to `dropA`, as Rust drops it;
// both closed as it's closed, which drops what they hold (ADR 0465). A
// step after its end steps each on, as Rust's `Zip`, which isn't fused,
// does.
function $lazyZip(a, b, dropA) {
  const left = a[Symbol.iterator]();
  const right = b[Symbol.iterator]();
  const close = () => {
    left.return?.();
    right.return?.();
  };
  const zipped = new $Steps(() => {
    const item = left.next();
    if (item.done) return item;
    const other = right.next();
    if (other.done) {
      dropA?.(item.value);
      return other;
    }
    return { done: false, value: [item.value, other.value] };
  }, close);
  zipped.from = [left, right];
  return zipped;
}

// `a.zip(b)` std reads by index, its sides `TrustedRandomAccess` and a
// step of `a` running code, a `map`'s closure: a pair of each's next while
// both have one, `a` never stepped once `b`, which knows its length, has
// none, as std's zip pairs them up to the shorter's length (ADR 0973).
function $trustedZip(a, b) {
  const left = a[Symbol.iterator]();
  const zipped = new $Steps(
    () => {
      if (b.len() === 0) return { done: true, value: undefined };
      const item = left.next();
      if (item.done) return item;
      return { done: false, value: [item.value, b.next().value] };
    },
    () => {
      left.return?.();
      b.return?.();
    },
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

// `a.zip(b).rev()` of two iterators that know their backs and lengths,
// `$iter`s: the longer trimmed from its back to the shorter's length, then
// each's back paired, as Rust's `Zip::next_back` does (ADR 0493).
function* $zipBack(a, b) {
  for (let n = a.len() - b.len(); n > 0; n--) a.back();
  for (let n = b.len() - a.len(); n > 0; n--) b.back();
  while (true) {
    const x = a.back();
    const y = b.back();
    if (x.done || y.done) return;
    yield [x.value, y.value];
  }
}
