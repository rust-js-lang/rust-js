// `r.unwrap_err()` or `r.expect_err(message)`: what an `Err` holds; of an
// `Ok`, a panic whose message shows its value by `debug`, the value then
// dropped by `drop`, where it has a destructor, as `$unwrapOk`'s (#1505).
function $unwrapErr(result, message = "called `Result::unwrap_err()` on an `Ok` value", debug = $debug, drop) {
  if (result.TAG === "Ok") {
    const panic = new Error(message + ": " + debug(result._0));
    if (drop !== undefined) $dropsUnwinding([() => drop(result._0)]);
    throw panic;
  }
  return result._0;
}
