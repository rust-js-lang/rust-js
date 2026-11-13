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
];
