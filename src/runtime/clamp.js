// `x.clamp(min, max)` of an integer, a number or a BigInt: `min` or `max` if
// it's past either. Bounds the wrong way round panic as std's integers do,
// each shown as `{:?}` shows it, which is its digits.
function $clamp(x, min, max) {
  if (!(min <= max)) throw new Error("min > max. min = " + min + ", max = " + max);
  return x < min ? min : x > max ? max : x;
}

// `x.clamp(min, max)` of what `cmp` orders, `Ord`'s: `min` or `max` if it's
// past either. Bounds the wrong way round panic as `Ord`'s default does,
// or, given `show`, as a `char`'s does, each as `{:?}` shows it. Of a type
// with a destructor, `drop`: the two not given back dropped, the later
// parameter first, and all three as a comparison or the bounds' panic
// unwinds, as std's `clamp` drops its parameters (#912).
function $clampBy(x, min, max, cmp, show, drop) {
  const parts = [x, min, max];
  const at = $unwinds(
    () => {
      if (cmp(min, max) > 0) {
        throw new Error(show ? `min > max. min = ${show(min)}, max = ${show(max)}` : "assertion failed: min <= max");
      }
      return cmp(x, min) < 0 ? 1 : cmp(x, max) > 0 ? 2 : 0;
    },
    drop && (() => drop(max)),
    drop && (() => drop(min)),
    drop && (() => drop(x)),
  );
  if (drop !== undefined) $inTurn(...[2, 1, 0].filter((i) => i !== at).map((i) => () => drop(parts[i])));
  return parts[at];
}

// `a.max(b)` and `a.min(b)` of a type with a destructor, std's `Ord`
// defaults': `b` against `a`, `cmp(b, a) < 0`, picks; the one not given
// back dropped by `drop`, and both, `b` first, as the comparison panics, as
// std's drops its parameters (#912).
function $maxOf(a, b, cmp, drop) {
  return $keptOf(a, b, cmp, drop, true);
}

function $minOf(a, b, cmp, drop) {
  return $keptOf(a, b, cmp, drop, false);
}

function $keptOf(a, b, cmp, drop, max) {
  const less = $unwinds(
    () => cmp(b, a) < 0,
    () => drop(b),
    () => drop(a),
  );
  const [kept, other] = less === max ? [a, b] : [b, a];
  drop(other);
  return kept;
}
