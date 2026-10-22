// A `String`, or a `Formatter`'s text, as a `fmt::Write` generic code is
// given, in a box: what it writes is added to the box's `value`, which the
// caller takes back (ADR 0180). A `char` and a `write!`'s text are strings.
const $stringWriter = {
  writeStr(w, s) {
    w.value += s;
  },
  writeChar(w, c) {
    w.value += c;
  },
  writeFmt(w, text) {
    w.value += text;
  },
};

// A writer's dictionary through the box generic code is given a `&mut`
// to it in, `W = &mut s` taken by value, `writer: impl Write`, which std's
// `impl Write for &mut W` writes through: the writer's own, of what's in
// the box, `$mutWriter($stringWriter)` of a `String` (ADRs 0180, 0454).
function $mutWriter(writer) {
  return {
    writeStr: (w, s) => writer.writeStr(w.value, s),
    writeChar: (w, c) => writer.writeChar(w.value, c),
    writeFmt: (w, text) => writer.writeFmt(w.value, text),
  };
}
