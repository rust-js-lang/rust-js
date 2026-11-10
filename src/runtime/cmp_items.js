
function $cmpItems(a, b, cmp) {
  for (let i = 0; i < a.length && i < b.length; i++) {
    const order = cmp(a[i], b[i]);
    if (order !== 0) {
      return order;
    }
  }
  return $cmp(a.length, b.length);
}
// `a.eq(b)` of two iterators: each item in turn, as Rust steps them, till
// one differs or either ends; by `eq`, or `===` of what JS compares so. Of
// items with a destructor, each compared pair dropped, `b`'s first, and one
// left where the other ended; each closed as it ends, which drops what it
// holds, `b` first, as std's are dropped (ADR 0465): not as either's
// step ends, `$comparedOf`'s, but after the item taken of the other is
// dropped, as std's `iter_compare` frame drops what it took (#1145). `a` is stepped by its
// `tryFold`, as std's `iter_compare` steps it by `try_for_each`, and `b` by
// its `next`. Of two `TrustedLen` ones, given their `lengths`, std's
// `SpecIterEq`: of unequal ones, `false`, with none of their items stepped
// or compared, each closed, `b` first (ADR 0992).
function $iterEq(a, b, eq, dropA, dropB, lengths) {
  const x = $comparedOf(a);
  const y = $comparedOf(b);
  return $endsAfter(() => $closeEach(y, x), () => {
    if (lengths !== undefined && lengths[0] !== lengths[1]) return false;
    let same = true;
    const stopped = $lent(x).tryFold(
      false,
      (_, p) => {
        // Its step, which may panic: `p`, the closure's, dropped as it
        // unwinds, before what's closed (#1145).
        const q = $unwinds(() => y.next(), dropA && (() => dropA(p)));
        if (q.done) {
          dropA?.(p);
          same = false;
          return true;
        }
        same = eq ? eq(p, q.value) : p === q.value;
        dropB?.(q.value);
        dropA?.(p);
        return !same;
      },
      (stop) => stop,
    );
    if (stopped) return same;
    const q = y.next();
    if (q.done) return true;
    dropB?.(q.value);
    return false;
  });
}

// `a.eq(b)` of generic code, each side's `LengthFirst` its caller's (#1182):
// where both are `TrustedLen`, std's `SpecIterEq` compares their lengths
// first, each read where JS knows it, `true`, or of one collected first,
// `"pure"`, whose steps run nothing and drop nothing; else item by item.
function $iterEqGiven(a, b, eq, dropA, dropB, firstA, firstB) {
  if (!firstA || !firstB) return $iterEq(a, b, eq, dropA, dropB);
  const [x, n] = $lengthFirst(a, firstA);
  const [y, m] = $lengthFirst(b, firstB);
  return $iterEq(x, y, eq, dropA, dropB, [n, m]);
}

// `it`, of a `TrustedLen` iterator, and how many it has left
// (`$iterEqGiven`): its `len()` or its exact hint, or, of one `"pure"`, what
// it gives, collected.
function $lengthFirst(it, first) {
  if (first === "pure") {
    const items = $foldToArray(it);
    return [$iter(items), items.length];
  }
  if (typeof it.len === "function") return [it, it.len()];
  const [lower, upper] = $sizeHint(it);
  if (lower !== upper) throw new Error("rust-js: a `TrustedLen` iterator's length isn't known (#1182)");
  return [it, lower];
}
