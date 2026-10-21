// An integer's `{:?}` in a crate that asks `{:x?}` (ADR 0559): its decimal
// `text`, or, where the options say, its `hex` digits, upper-case for
// `{:X?}`, after `0x` where it's alternate, as `{:#x}` shows it.
function $debugInt(text, hex, options) {
  const asked = options?.debugHex;
  if (asked == null) return text;
  return (options.alternate ? "0x" : "") + (asked === "X" ? hex.toUpperCase() : hex);
}
