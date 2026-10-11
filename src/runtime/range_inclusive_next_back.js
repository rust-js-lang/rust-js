// A `RangeInclusive`'s `next_back()`, `{ start, end }`: its end, moved
// down, while it's at least its start; a `char`'s by its code point, and
// before the first `char`, its end its start's and its start past it.
function $rangeInclusiveNextBack(range) {
  if (typeof range.start === "string") {
    if (range.start.codePointAt(0) > range.end.codePointAt(0)) return undefined;
    const c = range.end;
    const before = $charStep(c, -1);
    if (before === undefined) range.start = $charStep(c, 1);
    else range.end = before;
    return c;
  }
  return range.start <= range.end ? range.end-- : undefined;
}
