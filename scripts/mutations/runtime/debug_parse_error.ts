// Mutations of crates/rust-js-program/src/runtime/debug_parse_error.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-from-int-debug-shows-message",
    breaks: "`{:?}` of a `TryFromIntError` shows its message, not its kind, `TryFromIntError(PosOverflow)`",
    file: "crates/rust-js-program/src/runtime/debug_parse_error.js",
    find: "  const kind = $parseErrorKind(message);\n",
    replace: "  const kind = name === \"TryFromIntError\" ? message : $parseErrorKind(message);\n",
    tests: ["test/corpus.test.ts", "-t", "std_errors_boxed"],
  },
  {
    name: "char-try-from-error-debug-kind",
    breaks: "`{:?}` of a `CharTryFromError` shows a kind it hasn't, not `CharTryFromError(())`",
    file: "crates/rust-js-program/src/runtime/debug_parse_error.js",
    find: "  if (name === \"CharTryFromError\" || name === \"TryFromCharError\") return",
    replace: "  if (false) return",
    tests: ["test/corpus.test.ts", "-t", "^char_try_from.rs$"],
  },
  {
    name: "write-try-from-int-error-of-unit",
    breaks: "a `TryFromIntError`'s `Debug` written into a writer shows `()` where Rust shows its kind",
    file: "crates/rust-js-program/src/runtime/debug_parse_error.js",
    find: "  if (name === \"TryFromIntError\") return $writeDebugTuple(f, name, [kind], pretty);\n",
    replace: "  if (name === \"TryFromIntError\") return $writeDebugTuple(f, name, [[undefined, (g) => g.write(\"()\")]], pretty);\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_rest.rs$"],
  },
  {
    name: "borrow-error-debug-shows-a-kind",
    breaks: "a `RefCell`'s `BorrowError` and `BorrowMutError` shown by `Debug` write a `kind` field std's don't, `BorrowError { kind: undefined }` (#2476)",
    file: "crates/rust-js-program/src/runtime/debug_parse_error.js",
    find: 'const $namedAlone = new Set(["ParseBoolError", "BorrowError", "BorrowMutError"]);\n',
    replace: 'const $namedAlone = new Set(["ParseBoolError"]);\n',
    tests: ["test/corpus.test.ts", "-t", "^borrow_errors_shown_by_debug.rs$"],
  },
];
