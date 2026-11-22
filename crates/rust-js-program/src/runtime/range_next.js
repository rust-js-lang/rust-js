// A `Range`'s `next()`, `{ start, end }`: its start, moved past, while it's
// before its end; a `char`'s by its code point.
function $rangeNext(range) {
  if (typeof range.start === "string") {
    if (range.start.codePointAt(0) >= range.end.codePointAt(0)) return undefined;
    const c = range.start;
    range.start = $charStep(c, 1);
    return c;
  }
  return range.start < range.end ? range.start++ : undefined;
}
