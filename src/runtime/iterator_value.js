// What a JS iterator steps, as its Rust type has it: a `$iterator`'s value,
// `of`, the crate's own iterator or a range; or the JS iterator itself, std's
// (ADR 0473).
function $iteratorValue(it) {
  return "of" in it ? it.of : it;
}
