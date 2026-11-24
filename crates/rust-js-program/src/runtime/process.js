// The process a program runs in, Node's (ADR 0391): what std asks of it,
// each through `$process`, which in a browser, that has none, panics,
// saying what needed it.
function $process(what) {
  const process = globalThis.process;
  if (process?.argv === undefined) {
    // its own Error: a browser has no process, which Node's functions need (ADR 0391).
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
// `env::temp_dir()`, as std's unix body: `TMPDIR`, set, if empty too; else
// of macOS `confstr(_CS_DARWIN_USER_TEMP_DIR)`, which `getconf` prints, or
// `/tmp` where it fails, as std's does; of Android `/data/local/tmp`; of
// another `/tmp` (#1639).
function $tempDir() {
  const process = $process("`std::env::temp_dir`");
  const dir = process.env.TMPDIR;
  if (dir !== undefined) return dir;
  if (process.platform === "android") return "/data/local/tmp";
  if (process.platform !== "darwin") return "/tmp";
  const asked = process
    .getBuiltinModule("node:child_process")
    .spawnSync("getconf", ["DARWIN_USER_TEMP_DIR"], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  if (asked.error !== undefined || asked.status !== 0) return "/tmp";
  return asked.stdout.endsWith("\n") ? asked.stdout.slice(0, -1) : asked.stdout;
}
// `env::home_dir()`, as std's unix body: `HOME` where it isn't empty, else
// the password database's entry of the process's user, `getpwuid_r` of
// `getuid()`, or `None`, `undefined`, where it has none, or on Android.
// Node's `os.userInfo()` reads `geteuid()`'s entry, the same user unless
// the process runs set-user-ID, which it says (#1639).
function $homeDir() {
  const process = $process("`std::env::home_dir`");
  const home = process.env.HOME;
  if (home !== undefined && home !== "") return home;
  if (process.platform === "android") return undefined;
  if (process.getuid() !== process.geteuid()) {
    // its own Error: the real user's entry, which Node can't ask of a set-user-ID process (#1639).
    throw new Error("`std::env::home_dir` reads the real user's entry, which Node can't ask of a set-user-ID process (#1639)");
  }
  const user = $fsTry(() => process.getBuiltinModule("node:os").userInfo());
  return user.TAG === "Ok" ? user._0.homedir : undefined;
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
