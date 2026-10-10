// Mutations of src/runtime/ascii_case.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "ascii-case-unicode",
    breaks: "`to_ascii_lowercase` lowers every letter, `Ü` too, as JS's `toLowerCase` does",
    file: "src/runtime/ascii_case.js",
    find: "    : text.replace(/[A-Z]+/g, (letters) => letters.toLowerCase());",
    replace: "    : text.toLowerCase();",
    tests: ["test/corpus.test.ts","-t","std_methods"],
  },
  {
    name: "ascii-case-range-ignored",
    breaks: "`s[a..b].make_ascii_uppercase()` changes all of `s`",
    file: "src/runtime/ascii_case.js",
    find: "    return $strSlice(text, 0, start) + $asciiCase(part, upper) + $strSlice(text, end);",
    replace: "    return $asciiCase(text, upper);",
    tests: ["test/corpus.test.ts","-t","str_mut"],
  },
  {
    name: "ascii-case-range-checked-late",
    breaks: "`s[a..].make_ascii_uppercase()` inside a `char` says its end, not its start, is",
    file: "src/runtime/ascii_case.js",
    find: "    const part = $strSlice(text, start, end);\n    return $strSlice(text, 0, start) + $asciiCase(part, upper) + $strSlice(text, end);",
    replace: "    const before = $strSlice(text, 0, start);\n    return before + $asciiCase($strSlice(text, start, end), upper) + $strSlice(text, end);",
    tests: ["test/corpus.test.ts","-t","str_mut_boundary"],
  },
];
