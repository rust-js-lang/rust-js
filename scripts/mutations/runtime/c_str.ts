// Mutations of src/runtime/c_str.js (ADR 1119).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "c-str-interior-nul-unchecked",
    breaks: "`CStr::from_bytes_with_nul(b\"a\\0b\\0\")` is `Ok`",
    file: "src/runtime/c_str.js",
    find: "  if (nul === bytes.length - 1) return { TAG: \"Ok\", _0: bytes };\n  return { TAG: \"Err\", _0: nul < 0 ? \"NotNulTerminated\"",
    replace: "  if (nul >= 0) return { TAG: \"Ok\", _0: bytes };\n  return { TAG: \"Err\", _0: nul < 0 ? \"NotNulTerminated\"",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-str-until-nul-whole",
    breaks: "`CStr::from_bytes_until_nul` keeps what's past the first nul",
    file: "src/runtime/c_str.js",
    find: "  return { TAG: \"Ok\", _0: nul === bytes.length - 1 ? bytes : $part(bytes, 0, nul + 1) };\n",
    replace: "  return { TAG: \"Ok\", _0: bytes };\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-string-new-unended",
    breaks: "`CString::new(..)` gives its bytes no nul",
    file: "src/runtime/c_str.js",
    find: "  return { TAG: \"Ok\", _0: bytes.concat([0]) };\n",
    replace: "  return { TAG: \"Ok\", _0: bytes };\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-string-into-bytes-in-place",
    breaks: "`Vec::from(s.clone())` takes the nul off `s` too, a clone never made sharing its array",
    file: "src/runtime/c_str.js",
    find: "  return c.slice(0, -1);\n",
    replace: "  c.pop();\n  return c;\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "debug-c-str-ascii-unescaped",
    breaks: "`{:?}` of a C string writes a tab as it is, where std's is `\\t`",
    file: "src/runtime/c_str.js",
    find: "code < 0x80 ? ascii([code])",
    replace: "code < 0x80 ? ch",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "debug-c-str-invalid-dropped",
    breaks: "`{:?}` of a C string drops a byte that isn't UTF-8, where std's is `\\xff`",
    file: "src/runtime/c_str.js",
    find: "    out += ascii(invalid);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-string-with-nul-interior-unchecked",
    breaks: "`CString::from_vec_with_nul(b\"a\\0b\\0\")` is `Ok`",
    file: "src/runtime/c_str.js",
    find: "  if (nul === bytes.length - 1) return { TAG: \"Ok\", _0: bytes };\n  return { TAG: \"Err\", _0: { errorKind",
    replace: "  if (nul >= 0) return { TAG: \"Ok\", _0: bytes };\n  return { TAG: \"Err\", _0: { errorKind",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-str-error-pretty-flat",
    breaks: "`{:#?}` of a `NulError` is on one line",
    file: "src/runtime/c_str.js",
    find: "      if (!pretty) return `NulError(",
    replace: "      if (true) return `NulError(",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
];
