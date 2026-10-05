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

// A `&mut String` or `&mut Formatter` taken by value, `writer: impl Write`,
// which std's `impl Write for &mut W` writes through: generic code gives
// a `&mut` to the writer, whose `value` is the caller's box (ADR 0180).
const $mutStringWriter = {
  write_str(w, s) {
    w.value.value += s;
  },
  write_char(w, c) {
    w.value.value += c;
  },
  write_fmt(w, text) {
    w.value.value += text;
  },
};
