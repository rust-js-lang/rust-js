// A comparison's `Ordering`, -1, 0 or 1, as JS's `<` orders numbers and
// `bool`s. Strings and `char`s by code point, as Rust orders them, where
// `<` compares UTF-16 units: where they first differ, a surrogate, of a
// character past U+FFFF, comes after a unit from U+E000 by code point, and
// before it by unit (ADR 0183).
function $cmp(a, b) {
  if (typeof a === "string") {
    let i = 0;
    while (i < a.length && i < b.length && a.charCodeAt(i) === b.charCodeAt(i)) i++;
    if (i < a.length && i < b.length) {
      const x = a.charCodeAt(i);
      const y = b.charCodeAt(i);
      if (x >= 0xd800 && y >= 0xd800 && x < 0xe000 !== y < 0xe000) return x < 0xe000 ? 1 : -1;
      return x < y ? -1 : 1;
    }
  }
  return a < b ? -1 : a > b ? 1 : 0;
}
