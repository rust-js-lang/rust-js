// Where the byte `at` of `s` is, in UTF-16 units: if a `char` starts there,
// or it's the end. Otherwise `undefined`, as `is_char_boundary` is `false`.
// Of a long text, from its facts (#1184): of one all ASCII, `at` itself; of
// a part of another, that one's; else from the step of bytes before it.
function $charBoundary(s, at) {
  let bytes = 0;
  let unit = 0;
  if (s.length >= $textLong) {
    const facts = $textFacts(s);
    if (at >= facts.bytes) return at === facts.bytes ? s.length : undefined;
    if (facts.ascii) return at;
    if (facts.root !== undefined) {
      const inRoot = $charBoundary(facts.root, facts.rootByte + at);
      return inRoot === undefined ? undefined : inRoot - facts.rootUnit;
    }
    const stops = $textStops(s, facts);
    const step = Math.floor(at / $textStep);
    bytes = stops[2 * step];
    unit = stops[2 * step + 1];
  }
  while (unit < s.length) {
    if (bytes === at) return unit;
    if (bytes > at) return undefined;
    const code = s.charCodeAt(unit);
    const pair = code >= 0xd800 && code < 0xdc00;
    bytes += code < 0x80 ? 1 : code < 0x800 ? 2 : pair ? 4 : 3;
    unit += pair ? 2 : 1;
  }
  return bytes === at ? unit : undefined;
}
