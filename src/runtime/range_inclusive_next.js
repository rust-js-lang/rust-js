// A `RangeInclusive`'s `next()`, `{ start, end }`: its start, moved past,
// while it's at most its end. Past its end, as Rust's, it gives nothing; a
// `char`'s by its code point, and past the last `char`, its start its end's
// and its end before it.
function $rangeInclusiveNext(range) {
  if (typeof range.start === "string") {
    if (range.start.codePointAt(0) > range.end.codePointAt(0)) return undefined;
    const c = range.start;
    const next = $charStep(c, 1);
    if (next === undefined) range.end = $charStep(c, -1);
    else range.start = next;
    return c;
  }
  return range.start <= range.end ? range.start++ : undefined;
}
