// A long text's UTF-8 facts, kept by its text for the few read last
// (#1184): its length in bytes, whether it's all ASCII, where each step of
// `$textStep` bytes starts, and its bytes, each made once where it's first
// asked; or, of a part `&s[a..b]` of another, where it is in that one, its
// `root`, whose facts are its own. Rust's `len()`, `as_bytes()` and
// `&s[i..]` cost nothing to ask again, and a parser asks them each step:
// JS's text has no bytes to ask, so each counted them again, and a loop
// over a long text was quadratic. Text is a value, never changed, so what's
// kept of it is always its own. Text shorter than `$textLong` units is
// counted each time, in fewer steps than a lookup; the facts of at most
// `$textKept` long ones are kept, the one asked longest ago forgotten first.
const $textLong = 64;
const $textKept = 8;
const $textStep = 64;
const $textFactsKept = new Map();

function $textFacts(s) {
  let facts = $textFactsKept.get(s);
  if (facts !== undefined) {
    // The one asked last is the last one kept, forgotten last.
    $textFactsKept.delete(s);
    $textFactsKept.set(s, facts);
    return facts;
  }
  const bytes = $bytesCounted(s);
  return $textKeep(s, { bytes, ascii: bytes === s.length, stops: undefined, utf8: undefined, root: undefined, rootByte: 0, rootUnit: 0 });
}

function $textKeep(s, facts) {
  $textFactsKept.set(s, facts);
  if ($textFactsKept.size > $textKept) $textFactsKept.delete($textFactsKept.keys().next().value);
  return facts;
}

// The facts of `part`, a long text that's `s`'s from its byte `byte` and
// unit `unit` on, `bytes` long, of `s`'s, which needn't count it again.
function $textPart(part, s, byte, unit, bytes) {
  if ($textFactsKept.has(part)) return;
  const of = $textFacts(s);
  const root = of.ascii ? undefined : (of.root ?? s);
  $textKeep(part, { bytes, ascii: of.ascii, stops: undefined, utf8: undefined, root, rootByte: of.rootByte + byte, rootUnit: of.rootUnit + unit });
}

function $bytesCounted(s) {
  let bytes = 0;
  for (let i = 0; i < s.length; i++) {
    const unit = s.charCodeAt(i);
    if (unit < 0x80) bytes++;
    else if (unit < 0x800) bytes += 2;
    else if (unit >= 0xd800 && unit < 0xdc00) {
      bytes += 4;
      i++;
    } else bytes += 3;
  }
  return bytes;
}

// `s.as_bytes()`: its UTF-8 bytes, lent, never written: frozen, so a write
// is an error, not a change to what another reads. A long text's are made
// once, kept with its facts; where they're owned, `to_vec()`'s, they're
// copied, as a slice lent is.
function $utf8Bytes(s) {
  if (s.length < $textLong) return Object.freeze(Array.from(new TextEncoder().encode(s)));
  const facts = $textFacts(s);
  return (facts.utf8 ??= Object.freeze(Array.from(new TextEncoder().encode(s))));
}

function $byteLen(s) {
  return s.length < $textLong ? $bytesCounted(s) : $textFacts(s).bytes;
}

// Of a long text that isn't all ASCII, where each step of `$textStep`
// bytes is: the byte and the unit the character at or before it starts at,
// `[byte, unit, byte, unit, ...]`, so the unit of a byte is found from the
// step before it, not from the text's start.
function $textStops(s, facts) {
  if (facts.stops !== undefined) return facts.stops;
  const stops = new Uint32Array(2 * (Math.floor(facts.bytes / $textStep) + 1));
  let bytes = 0;
  let next = 0;
  for (let unit = 0; unit < s.length; ) {
    const code = s.charCodeAt(unit);
    const width = code < 0x80 ? 1 : code < 0x800 ? 2 : code >= 0xd800 && code < 0xdc00 ? 4 : 3;
    while (next * $textStep < bytes + width) {
      stops[2 * next] = bytes;
      stops[2 * next + 1] = unit;
      next++;
    }
    bytes += width;
    unit += width === 4 ? 2 : 1;
  }
  for (; 2 * next < stops.length; next++) {
    stops[2 * next] = bytes;
    stops[2 * next + 1] = s.length;
  }
  return (facts.stops = stops);
}
