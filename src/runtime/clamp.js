// `x.clamp(min, max)` of an integer, a number or a BigInt: `min` or `max` if
// it's past either. Bounds the wrong way round panic as std's integers do,
// each shown as `{:?}` shows it, which is its digits.
function $clamp(x, min, max) {
  if (!(min <= max)) throw new Error("min > max. min = " + min + ", max = " + max);
  return x < min ? min : x > max ? max : x;
}

// `x.clamp(min, max)` of what `ord` orders, a `PartialOrd` dictionary, by
// std's `min <= max`, `x < min` and `x > max` (#937): `min` or `max` if it's
// past either. Bounds the wrong way round panic as `Ord`'s default does,
// or, given `show`, as a `char`'s does, each as `{:?}` shows it. Of a type
// with a destructor, `drop`: the two not given back dropped, the later
// parameter first, and all three as a comparison or the bounds' panic
// unwinds, as std's `clamp` drops its parameters (#912).
function $clampBy(x, min, max, ord, show, drop) {
  const parts = [x, min, max];
  const at = $unwinds(
    () => {
      if (!ord.le(min, max)) {
        throw new Error(show ? `min > max. min = ${show(min)}, max = ${show(max)}` : "assertion failed: min <= max");
      }
      return ord.lt(x, min) ? 1 : ord.gt(x, max) ? 2 : 0;
    },
    drop && (() => drop(max)),
    drop && (() => drop(min)),
    drop && (() => drop(x)),
  );
  if (drop !== undefined) $inTurn(...[2, 1, 0].filter((i) => i !== at).map((i) => () => drop(parts[i])));
  return parts[at];
}

// `a.max(b)` and `a.min(b)` of a type with a destructor, std's `Ord`
// defaults': `b` against `a`, `ord.lt(b, a)`, `PartialOrd`'s, picks (#937); the one not given
// back dropped by `drop`, and both, `b` first, as the comparison panics, as
// std's drops its parameters (#912).
function $maxOf(a, b, ord, drop) {
  return $keptOf(a, b, ord, drop, true);
}

function $minOf(a, b, ord, drop) {
  return $keptOf(a, b, ord, drop, false);
}

function $keptOf(a, b, ord, drop, max) {
  const less = $unwinds(
    () => ord.lt(b, a),
    () => drop(b),
    () => drop(a),
  );
  const [kept, other] = less === max ? [a, b] : [b, a];
  drop(other);
  return kept;
}
