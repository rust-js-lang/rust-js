// A JS iterator of `f` of what `it` steps, `$iteratorValue`'s, stepped as
// `it` is: a `$iterator`'s made again, `with`, of a clone of its value
// (ADR 0473).
function $iteratorWith(it, f) {
  return "of" in it ? it.with(f(it.of)) : f(it);
}
