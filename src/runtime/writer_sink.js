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
// std's `write_fmt` of a writer whose `write_fmt` is std's, its
// dictionary's `writeFmt` (#1652, step 6 of ADR 1135): `fmt::write` of
// `args`, the function of its `Arguments`, into its sink, each piece a
// call of `writeStr` or `writeChar`.
function $writerWriteFmt(writeStr, writeChar) {
  return (w, args) => $writingInto($writerSink(w, writeStr, writeChar), args);
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
function $writePadded(f, s, options = {}) {
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
// An integer's `Debug` into a writer, given its `Formatter`'s options as
// the writer it's shown in has them: its `LowerHex`, `lower` its digits,
// or `UpperHex`, `upper`, where `{:x?}` or `{:X?}` asks, else its `Display`.
function $writeDebugInteger(f, n, lower, upper, options = {}) {
  if (options.debugHex === "x") return $writeIntegral(f, true, "0x", lower, options);
  if (options.debugHex === "X") return $writeIntegral(f, true, "0x", upper, options);
  return $writeInteger(f, n, options);
}
// A `char`'s `Display` into a writer, given its `Formatter`'s options as
// the writer it's shown in has them: its `write_char` where it has no width
// and no precision, else `pad` of its text.
function $writeCharOf(f, c, options = {}) {
  if (options.width === undefined && options.precision === undefined) return f.writeChar(c);
  return $writePadded(f, c, options);
}
// A writer of the crate's own shown into a writer, `f`, a piece at a time
// (#1652, ADR 1135): `writer` called with `args`, the sink armed for it.
// Its arguments are evaluated before it's armed, and a writer that writes
// through a sink makes it, `$sink()`, as its first statement, so that call
// claims it; any `$sink()` after, of a `format!` or a `to_string()` the
// writer makes, is a string of its own.
// A writer that claims it gives back nothing of its own, its sink's
// `done()`, `""`; one that gives back its text didn't claim it, and a writer
// it called claimed it in its place, so what was written isn't its text: an
// error, never that text (#1735).
let $writerArmed;
function $writingInto(f, writer, ...args) {
  const outer = $writerArmed;
  $writerArmed = f;
  let given;
  try {
    given = writer(...args);
  } finally {
    const unclaimed = $writerArmed === f;
    $writerArmed = outer;
    if (unclaimed) throw new Error("rust-js: a writer shown into a writer didn't write through its sink");
  }
  if (typeof given === "string" && given !== "") {
    throw new Error("rust-js: a writer shown into a writer didn't write through its sink");
  }
}
// A `str`'s `Debug` into a writer, as std's writes it: its quote a
// `write_char`; each run of `char`s shown as they are one `write_str`, then
// each escape one, `\"` or `\u{301}`; the rest one, `""` too; then its
// closing quote. Its width and precision change nothing.
function $writeDebugStr(f, s) {
  f.writeChar('"');
  let run = "";
  for (const c of s) {
    const escaped = $debugChar(c, '"');
    if (escaped === c) {
      run += c;
      continue;
    }
    f.write(run);
    f.write(escaped);
    run = "";
  }
  f.write(run);
  f.writeChar('"');
}
// A `char`'s `Debug` into a writer: its quotes and itself, each a
// `write_char`, or its escape, one `write_str`, between them.
function $writeDebugChar(f, c) {
  f.writeChar("'");
  const escaped = $debugChar(c, "'");
  if (escaped === c) f.writeChar(c);
  else f.write(escaped);
  f.writeChar("'");
}
