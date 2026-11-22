// Mutations of crates/rust-js-program/src/runtime/process.js (ADR 0391).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "process-ungated",
    breaks: "in a browser, `env::args()` throws JS's own `TypeError`, not a panic naming what needed Node",
    file: "crates/rust-js-program/src/runtime/process.js",
    find: "  if (process?.argv === undefined) {\n",
    replace: "  if (false) {\n",
    tests: ["test/process.test.ts", "-t", "in a browser"],
  },
  {
    name: "args-with-node",
    breaks: "`env::args()` begins with Node's own path, not the program's",
    file: "crates/rust-js-program/src/runtime/process.js",
    find: "argv.slice(1)",
    replace: "argv.slice(0)",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
  },
  {
    name: "var-missing-unnamed",
    breaks: "a missing variable is an `Err` of no `VarError` std has, shown as a `NotUnicode`",
    file: "crates/rust-js-program/src/runtime/process.js",
    find: "{ TAG: \"Err\", _0: \"NotPresent\" }",
    replace: "{ TAG: \"Err\", _0: \"Missing\" }",
    tests: ["test/corpus.test.ts", "-t", "^env_vars.rs$"],
  },
  {
    name: "abort-exit-of-another-status",
    breaks: "an abort ends the process with 1, not 134, as Rust's SIGABRT gives a shell (#847)",
    file: "crates/rust-js-program/src/runtime/process.js",
    find: "  globalThis.process?.exit?.(code);\n",
    replace: "  globalThis.process?.exit?.(1);\n",
    tests: ["test/corpus.test.ts", "-t", "^unwinding_drop_aborts_field.rs$"],
  },
  {
    name: "current-dir-unwrapped",
    breaks: "`env::current_dir()` is the directory's text itself, no `Ok`, so `?` of it reads a string's `TAG` (#1420)",
    file: "crates/rust-js-program/src/runtime/process.js",
    find: "  return $fsTry(() => process.cwd());\n",
    replace: "  return process.cwd();\n",
    tests: ["test/corpus.test.ts", "-t", "^fs_canonicalize_and_current_dir.rs$"],
  },
  {
    name: "temp-dir-darwin-is-tmp",
    breaks: "`env::temp_dir()` on macOS with no `TMPDIR` is `/tmp`, not `confstr(_CS_DARWIN_USER_TEMP_DIR)`'s `/var/folders/../T/`, which std's gives (#1639)",
    file: "crates/rust-js-program/src/runtime/process.js",
    find: '  if (process.platform !== "darwin") return "/tmp";\n',
    replace: '  return "/tmp";\n',
    tests: ["test/process.test.ts", "-t", "env::temp_dir on macOS with no TMPDIR"],
  },
  {
    name: "home-dir-empty-taken",
    breaks: "`env::home_dir()` of an empty `HOME` is `Some(\"\")`, where std's reads the password database's entry (#1639)",
    file: "crates/rust-js-program/src/runtime/process.js",
    find: '  if (home !== undefined && home !== "") return home;\n',
    replace: '  if (home !== undefined) return home;\n',
    tests: ["test/process.test.ts", "-t", "env::temp_dir and env::home_dir"],
  },
  {
    name: "home-dir-no-fallback",
    breaks: "`env::home_dir()` with no `HOME` is `None`, where std's reads the password database's entry of the process's user (#1639)",
    file: "crates/rust-js-program/src/runtime/process.js",
    find: '  return user.TAG === "Ok" ? user._0.homedir : undefined;\n',
    replace: '  return undefined;\n',
    tests: ["test/process.test.ts", "-t", "env::temp_dir and env::home_dir"],
  },
];
