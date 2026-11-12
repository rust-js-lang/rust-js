// Mutations of src/runtime/json_value.js (ADR 0083).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-get-mut-unwritten",
    breaks: "a handle `get_mut` gives of a `Value`'s entry writes nothing back",
    file: "src/runtime/json_value.js",
    find: "set value(next) { items.set(key, next); }",
    replace: "set value(next) {}",
    tests: ["test/serde.test.ts", "-t", "changes to and from Null"],
  },
  {
    name: "value-writer-unrewound",
    breaks: "a `Value`'s writer keeps what it opened of a value whose write failed and was caught, so the value after is built inside it, where serde_json builds each apart and drops a failed one (#1545)",
    file: "src/runtime/json_value.js",
    find: "      stack.length = depth;\n",
    replace: "",
    tests: ["test/serde-protocol.test.ts"],
  },
  {
    name: "value-wide-int-unchecked",
    breaks: "a `Value` of a 128-bit integer a hand-written impl writes holds it whatever it is, an impossible `Number(u128::MAX)`, where serde_json's value serializer gives `number out of range` past an `i64` or a `u64` (#1547)",
    file: "src/runtime/json_value.js",
    find: "_0: wide === undefined ? $jsonInt(n) : $jsonWideInt(n) }",
    replace: "_0: $jsonInt(n) }",
    tests: ["test/serde-protocol.test.ts", "-t", "a 128-bit integer is written by each serializer"],
  },
  {
    name: "value-wide-int-signed-range",
    breaks: "a `Value` holds a 128-bit integer only where it fits in an `i64`, so `u64::MAX` written by `serialize_u128` is out of range, which serde_json's `u64` holds (#1547)",
    file: "src/runtime/json_value.js",
    find: "n > 18446744073709551615n",
    replace: "n > 9223372036854775807n",
    tests: ["test/serde-protocol.test.ts", "-t", "a 128-bit integer is written by each serializer"],
  },
];
