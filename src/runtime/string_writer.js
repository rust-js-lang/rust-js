// A `String`, or a `Formatter`'s text, as a `fmt::Write` generic code is
// given, in a box: what it writes is added to the box's `value`, which the
// caller takes back (ADR 0180). A `char` and a `write!`'s text are strings.
// A `Formatter` of a writer that writes through a sink is given as that
// sink, `$sink()`'s, whose `value` set writes what it adds: its
// `write_char` is the sink's, which a `write!` into a writer may have armed
// with its writer's, and a `write!`'s `Arguments` is written into it a piece
// at a time, as std's `Formatter` writes them (#1652, step 6 of ADR 1135).
const $stringWriter = {
  writeStr(w, s) {
    w.value += s;
  },
  writeChar(w, c) {
    if (w.writeChar !== undefined) w.writeChar(c);
    else w.value += c;
  },
  // Given the function of a `write!`'s `Arguments` where writes are
  // followed a piece at a time, its text, and what it wrote before a value
  // that failed, as std's writes each piece into the string; till ADR
  // 1135's last step, its text where they aren't.
  writeFmt(w, args) {
    if (typeof args !== "function") w.value += args;
    else if (w.writeChar !== undefined) $writingInto(w, args);
    else w.value += $fmtTextOr(args, (text) => (w.value += text));
  },
};

// A writer's dictionary through the box generic code is given a `&mut`
// to it in, `W = &mut s` taken by value, `writer: impl Write`, which std's
// `impl Write for &mut W` writes through: the writer's own, of what's in
// the box, `$mutWriter($stringWriter)` of a `String`, and `$mutWriter($vecWriter)`
// of an `io::Write`'s `Vec<u8>` (ADRs 0180, 0392, 0454).
function $mutWriter(writer) {
  return {
    writeStr: (w, s) => writer.writeStr(w.value, s),
    writeChar: (w, c) => writer.writeChar(w.value, c),
    writeFmt: (w, text) => writer.writeFmt(w.value, text),
    write: (w, bytes) => writer.write(w.value, bytes),
    writeAll: (w, bytes) => writer.writeAll(w.value, bytes),
    flush: (w) => writer.flush(w.value),
  };
}
