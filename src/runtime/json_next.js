// A hand-written visitor's `SeqAccess` and `MapAccess` (ADR 0459): what's
// next, a `Result` of an `Option`, `None` past the last, an element or a
// key given as `Some` boxed where it looks like `None` (ADR 0051).
function $jsonNextElement(seq, read, boxed) {
  return $jsonRead(seq, (s) => (s.next() ? (boxed ? $some(s.value(read)) : s.value(read)) : undefined));
}
function $jsonNextKey(map, read, boxed) {
  return $jsonRead(map, (m) => (m.next() ? (boxed ? $some(read(m.key())) : read(m.key())) : undefined));
}
function $jsonNextValue(map, read) {
  return $jsonRead(map, (m) => m.value(read));
}
// `next_entry`: its key and value, a pair, or `None` past the last.
function $jsonNextEntry(map, readKey, read) {
  return $jsonRead(map, (m) => (m.next() ? [readKey(m.key()), m.value(read)] : undefined));
}
