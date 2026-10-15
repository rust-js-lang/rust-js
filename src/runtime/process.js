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
// `process::exit(code)`.
function $exit(code) {
  $process("`std::process::exit`").exit(code);
}
