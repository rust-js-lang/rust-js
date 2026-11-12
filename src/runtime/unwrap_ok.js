// `r.unwrap()` or `r.expect(message)`: what an `Ok` holds; of an `Err`, a
// panic whose message shows its value by `debug`, the value then dropped by
// `drop`, where it has a destructor, as std's `unwrap_failed(msg, &e)`
// panics while `e` is a local of `unwrap`'s frame, which drops it as the
// panic unwinds, caught or not; a drop that panics then aborts (#1505).
function $unwrapOk(result, message = "called `Result::unwrap()` on an `Err` value", debug = $debug, drop) {
  if (result.TAG === "Err") {
    const panic = new Error(message + ": " + debug(result._0));
    if (drop !== undefined) $dropsUnwinding([() => drop(result._0)]);
    throw panic;
  }
  return result._0;
}
