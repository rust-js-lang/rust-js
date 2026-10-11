// A `char` range's step, a JS string's code point `by` one up or down,
// past the surrogates, which no `char` is: `undefined` past the first or
// the last `char`.
function $charStep(c, by) {
  let code = c.codePointAt(0) + by;
  if (code >= 0xd800 && code <= 0xdfff) code = by > 0 ? 0xe000 : 0xd7ff;
  return code < 0 || code > 0x10ffff ? undefined : String.fromCodePoint(code);
}
