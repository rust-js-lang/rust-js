
function $unwrapErr(result, message = "called `Result::unwrap_err()` on an `Ok` value", debug = $debug) {
  if (result.TAG === "Ok") {
    throw new Error(message + ": " + debug(result._0));
  }
  return result._0;
}
