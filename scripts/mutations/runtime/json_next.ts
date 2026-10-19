// Mutations of src/runtime/json_next.js (ADR 0459).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-next-element-unending",
    breaks: "`next_element()` past a sequence's last reads on, where it's `None`",
    file: "src/runtime/json_next.js",
    find: "  return $jsonRead(seq, (s) => (s.next() ? (boxed ? $some(s.value(read)) : s.value(read)) : undefined));\n",
    replace: "  return $jsonRead(seq, (s) => (s.next(), boxed ? $some(s.value(read)) : s.value(read)));\n",
    tests: ["test/serde.test.ts", "-t", "sequence and a map"],
  },
  {
    name: "json-next-entry-unpaired",
    breaks: "`next_entry()` gives its value alone, not its key and value",
    file: "src/runtime/json_next.js",
    find: "  return $jsonRead(map, (m) => (m.next() ? [readKey(m.key()), m.value(read)] : undefined));\n",
    replace: "  return $jsonRead(map, (m) => (m.next() ? (readKey(m.key()), m.value(read)) : undefined));\n",
    tests: ["test/serde.test.ts", "-t", "sequence and a map"],
  },
];
