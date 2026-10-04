// Mutations of src/lower/recognition/methods.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "str-bytes-unrecognized",
    breaks: "`s.bytes()` is an error, not the string's UTF-8 bytes",
    file: "src/lower/recognition/methods.rs",
    find: '        "as_bytes" | "bytes" if str => TextOp::Bytes,\n',
    replace: '        "as_bytes" if str => TextOp::Bytes,\n',
    tests: ["test/corpus.test.ts", "-t", "str_bytes"],
  },
  {
    name: "is-normal-unknown",
    breaks: "`x.is_normal()` of a float is an error",
    file: "src/lower/recognition/methods.rs",
    find: "        \"is_normal\" if float => NumOp::IsNormal { subnormal: false },\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "float_normal"],
  },
];
