// What a `write!` into a writer of the crate's own, or a `W: fmt::Write`'s,
// writes through (#1652): `write`, the writer's `write_str`, and
// `writeChar`, its `write_char`, each given `w`, the writer, and each piece
// as std's `write_fmt` gives it, a call at a time. Each throws the writer's
// `Err`, as `?` gives it on, so what's written after it isn't.
function $writerSink(w, writeStr, writeChar) {
  return {
    write: (s) => writeStr(w, s),
    writeChar: (c) => writeChar(w, c),
  };
}
// std's `padding`: `count` fill chars, the share `align` puts before the
// text, each a `write_char`; and how many go after it.
function $writePadding(f, count, align, fill) {
  const before = align === "Left" ? 0 : align === "Center" ? Math.floor(count / 2) : count;
  for (let i = 0; i < before; i++) f.writeChar(fill);
  return count - before;
}
// std's `pad` into a writer, `f.pad(s)`: `s` cut to its precision in
// `char`s, one `write_str`, with the fill chars that pad it to its width
// before and after it, each a `write_char`.
function $writePadded(f, s, options) {
  const { width, precision } = options;
  if (width === undefined && precision === undefined) return f.write(s);
  let chars = Array.from(s);
  if (precision !== undefined && chars.length > precision) {
    chars = chars.slice(0, precision);
    s = chars.join("");
  }
  if (width === undefined || chars.length >= width) return f.write(s);
  const after = $writePadding(f, width - chars.length, options.align ?? "Left", options.fill ?? " ");
  f.write(s);
  for (let i = 0; i < after; i++) f.writeChar(options.fill ?? " ");
}
// std's `pad_integral` into a writer, `f.pad_integral(nonNegative, prefix,
// digits)`: its sign a `write_char`, `-`, or `+` where `{:+}` asks; its
// prefix a `write_str` where `{:#}` asks, `""` too; its digits one; and
// padding to its width, each fill char a `write_char`: zeros after the sign
// and prefix where `{:0}` asks, else fill before them by its alignment.
function $writeIntegral(f, nonNegative, prefix, digits, options = {}) {
  let length = digits.length;
  const sign = !nonNegative ? "-" : options.plus ? "+" : undefined;
  if (sign !== undefined) length += 1;
  if (options.alternate) length += Array.from(prefix).length;
  const writePrefix = () => {
    if (sign !== undefined) f.writeChar(sign);
    if (options.alternate) f.write(prefix);
  };
  const width = options.width ?? 0;
  if (length >= width) {
    writePrefix();
    return f.write(digits);
  }
  const zero = options.zero === true;
  if (zero) writePrefix();
  const fill = zero ? "0" : (options.fill ?? " ");
  const after = $writePadding(f, width - length, zero ? "Right" : (options.align ?? "Right"), fill);
  if (!zero) writePrefix();
  f.write(digits);
  for (let i = 0; i < after; i++) f.writeChar(fill);
}
// An integer's `Display`, or `Debug`, into a writer: `pad_integral` of its
// digits, its sign apart.
function $writeInteger(f, n, options) {
  return $writeIntegral(f, n >= 0, "", String(n < 0 ? -n : n), options);
}
