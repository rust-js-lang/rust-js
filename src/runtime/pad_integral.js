// `f.pad_integral(nonNegative, prefix, digits)`, as std writes it given a
// `Formatter`'s options: its sign, a `+` where `{:+}` asks; its prefix where
// `{:#}` does; zeros between them and the digits where `{:0}` does, else
// padding as a number's (ADR 0143).
function $padIntegral(nonNegative, prefix, digits, options) {
  if (options === undefined) return (nonNegative ? "" : "-") + digits;
  const sign = !nonNegative ? "-" : options.plus ? "+" : "";
  const head = sign + (options.alternate ? prefix : "");
  const width = options.width;
  if (width === undefined) return head + digits;
  if (options.zero) return head + digits.padStart(width - [...head].length, "0");
  const align = { Left: "<", Right: ">", Center: "^" }[options.align] ?? ">";
  return $pad(head + digits, width, align, options.fill);
}
