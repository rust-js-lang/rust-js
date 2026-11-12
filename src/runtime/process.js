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
// Node's fs, `node:fs`, for std's files (#1156): as Node's process is
// asked, through `$process`, which in a browser panics, saying what needed
// it. Read where it's asked, never imported, so a browser's bundle loads
// without it.
function $nodeFs(what) {
  return $process(what).getBuiltinModule("node:fs");
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
// `env::current_dir()`: `Ok` of the working directory, or `Err` of the
// OS's error, of one that's gone.
function $currentDir() {
  const process = $process("`std::env::current_dir`");
  return $fsTry(() => process.cwd());
}
// `process::exit(code)`.
function $exit(code) {
  $process("`std::process::exit`").exit(code);
}
// An abort's end of the process, `code` its status, where there's one: a
// browser has none, so nothing ends, and `$abort` throws instead (#847).
// Not std's function, so not `$process`'s, which panics without one.
function $abortExit(code) {
  globalThis.process?.exit?.(code);
}
