// A `String`, or a `Formatter`'s text, as a `fmt::Write` generic code is
// given, in a box: what it writes is added to the box's `value`, which the
// caller takes back (ADR 0180). A `char` and a `write!`'s text are strings.
const $stringWriter = {
  write_str(w, s) {
    w.value += s;
  },
  write_char(w, c) {
    w.value += c;
  },
  write_fmt(w, text) {
    w.value += text;
  },
};

// A writer's dictionary through the box generic code is given a `&mut`
// to it in, `W = &mut s` taken by value, `writer: impl Write`, which std's
// `impl Write for &mut W` writes through: the writer's own, of what's in
// the box, `$mutWriter($stringWriter)` of a `String` (ADRs 0180, 0454).
function $mutWriter(writer) {
  return {
    write_str: (w, s) => writer.write_str(w.value, s),
    write_char: (w, c) => writer.write_char(w.value, c),
    write_fmt: (w, text) => writer.write_fmt(w.value, text),
  };
}
