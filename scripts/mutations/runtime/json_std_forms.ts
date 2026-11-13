// Mutations of src/runtime/json_std_forms.js, the checks of serde's own
// impls of std's types (#1623, ADR 1134).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-duration-overflow-unchecked",
    breaks: "a `Duration` whose nanoseconds carry past `u64::MAX` seconds is `Duration::new`'s panic, where serde's is `overflow deserializing Duration` (#1623)",
    file: "src/runtime/json_std_forms.js",
    find: '    throw $jsonError("overflow deserializing Duration");\n',
    replace: "",
    tests: ["test/serde-std.test.ts", "-t", "a Duration and a SystemTime"],
  },
  {
    name: "json-system-time-offset-unchecked",
    breaks: "a `SystemTime`'s offset that carries past `u64::MAX` seconds is `Duration::new`'s panic, where serde's is `overflow deserializing SystemTime epoch offset` (#1623)",
    file: "src/runtime/json_std_forms.js",
    find: '    throw $jsonError("overflow deserializing SystemTime epoch offset");\n',
    replace: "",
    tests: ["test/serde-std.test.ts", "-t", "a Duration and a SystemTime"],
  },
  {
    name: "json-system-time-held-as-u64",
    breaks: "a `SystemTime` of more seconds than unix's `i64` holds is read, where serde's is `overflow deserializing SystemTime` (#1623)",
    file: "src/runtime/json_std_forms.js",
    find: "offset / 1000000000n > 9223372036854775807n",
    replace: "offset / 1000000000n > 18446744073709551615n",
    tests: ["test/serde-std.test.ts", "-t", "a Duration and a SystemTime"],
  },
  {
    name: "json-ref-cell-borrow-unchecked",
    breaks: "a `RefCell` mutably borrowed is written, where serde's `try_borrow()` gives `already mutably borrowed` (#1623)",
    file: "src/runtime/json_std_forms.js",
    find: '  if (cell.borrows < 0) throw $jsonError("already mutably borrowed");\n',
    replace: "",
    tests: ["test/serde-std.test.ts", "-t", "a Cell, a RefCell, a Mutex and an RwLock"],
  },
  {
    name: "json-lock-poison-unchecked",
    breaks: "a poisoned `Mutex` is written, where serde's `lock()` gives `lock poison error while serializing` (#1623)",
    file: "src/runtime/json_std_forms.js",
    find: '    throw $jsonError("lock poison error while serializing");\n',
    replace: "",
    tests: ["test/serde-std.test.ts", "-t", "a Cell, a RefCell, a Mutex and an RwLock"],
  },
  {
    name: "json-lock-panic-unpoisoned",
    breaks: "a panic as a `Mutex`'s value is written leaves it unpoisoned, where its guard dropped as the panic unwinds poisons it (#1623)",
    file: "src/runtime/json_std_forms.js",
    find: "  const release = exclusive ? () => $unlock(cell) : () => $unborrow(cell);\n",
    replace: "  const release = () => $unborrow(cell);\n",
    tests: ["test/serde-std.test.ts", "-t", "a Cell, a RefCell, a Mutex and an RwLock"],
  },
  {
    name: "json-lock-error-poisons",
    breaks: "a write's error as a `Mutex`'s value is written poisons it, where it's a `Result`, its guard dropped as any is (#1623)",
    file: "src/runtime/json_std_forms.js",
    find: "  const written = $unwinds(() => $jsonTry(() => write(cell.value)), release);\n",
    replace: "  const written = { TAG: \"Ok\" }; $unwinds(() => write(cell.value), release);\n",
    tests: ["test/serde-std.test.ts", "-t", "a Cell, a RefCell, a Mutex and an RwLock"],
  },
];
