// The process a program runs in, Node's (ADR 0391): what std asks of it,
// each through `$process`, which in a browser, that has none, panics,
// saying what needed it.
function $process(what) {
  const process = globalThis.process;
  if (process?.argv === undefined) {
    throw new Error(`${what} needs Node's process, which a browser hasn't (ADR 0391)`);
  }
  return process;
}
// `env::args()`: the program's path, then its arguments, as `node app.js a` gives them.
function $args() {
  return $process("`std::env::args`").argv.slice(1);
}
// `env::var(key)`: `Ok` of the variable, or `Err` of `VarError::NotPresent`.
function $var(key) {
  const value = $process("`std::env::var`").env[key];
  return value === undefined ? { TAG: "Err", _0: "NotPresent" } : { TAG: "Ok", _0: value };
}
// `env::var_os(key)`: the variable, or `None`, `undefined`.
function $varOs(key) {
  return $process("`std::env::var_os`").env[key];
}
// `env::vars()`: each variable's name and value.
function $vars() {
  return Object.entries($process("`std::env::vars`").env);
}
// `env::vars_os()`: the same, an `OsString` its text (ADR 0173).
function $varsOs() {
  return Object.entries($process("`std::env::vars_os`").env);
}
// `process::exit(code)`.
function $exit(code) {
  $process("`std::process::exit`").exit(code);
}
