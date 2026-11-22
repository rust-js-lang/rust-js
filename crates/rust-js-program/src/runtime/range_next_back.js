// A `Range`'s `next_back()`, `{ start, end }`: its end, moved down, while
// it's past its start; a `char`'s by its code point.
function $rangeNextBack(range) {
  if (typeof range.start === "string") {
    if (range.start.codePointAt(0) >= range.end.codePointAt(0)) return undefined;
    range.end = $charStep(range.end, -1);
    return range.end;
  }
  return range.start < range.end ? --range.end : undefined;
}
