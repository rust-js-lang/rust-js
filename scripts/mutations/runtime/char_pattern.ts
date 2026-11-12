// Mutations of src/runtime/char_pattern.js (#118).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "char-matches-utf16-bytes",
    breaks: "`match_indices` of a predicate gives UTF-16 units, not UTF-8 bytes",
    file: "src/runtime/char_pattern.js",
    find: "    byte += $byteLen(c);\n",
    replace: "    byte += c.length;\n",
    tests: ["test/corpus.test.ts", "-t", "^char_patterns.rs$"],
  },
  {
    name: "rsplitn-by-unlimited",
    breaks: "`rsplitn(n, p)` splits at every match, not at most `n` pieces",
    file: "src/runtime/char_pattern.js",
    find: "k >= 0 && pieces.length < n - 1;",
    replace: "k >= 0;",
    tests: ["test/corpus.test.ts", "-t", "^char_patterns.rs$"],
  },
  {
    name: "split-terminator-by-keeps-end",
    breaks: "`split_terminator(p)` keeps the empty piece its end leaves",
    file: "src/runtime/char_pattern.js",
    find: "  if (pieces[pieces.length - 1] === \"\") pieces.pop();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^char_patterns.rs$"],
  },
  {
    name: "split-inclusive-by-trailing",
    breaks: "`split_inclusive(p)` gives an empty piece at the end",
    file: "src/runtime/char_pattern.js",
    find: "  if (piece !== \"\") pieces.push(piece);\n",
    replace: "  pieces.push(piece);\n",
    tests: ["test/corpus.test.ts", "-t", "^char_patterns.rs$"],
  },
  {
    name: "split-once-by-first",
    breaks: "`rsplit_once(p)` splits at the first match",
    file: "src/runtime/char_pattern.js",
    find: "  if (!last) {\n",
    replace: "  if (true) {\n",
    tests: ["test/corpus.test.ts", "-t", "^char_patterns.rs$"],
  },
  {
    name: "strip-by-untested",
    breaks: "`strip_prefix(p)` strips its first `char` whether `p` holds of it or not",
    file: "src/runtime/char_pattern.js",
    find: "  if (c === undefined || !test(c)) return undefined;\n",
    replace: "  if (c === undefined) return undefined;\n",
    tests: ["test/corpus.test.ts", "-t", "^char_patterns.rs$"],
  },
  {
    name: "replace-by-unlimited",
    breaks: "`replacen(p, to, n)` replaces every match",
    file: "src/runtime/char_pattern.js",
    find: "    if (count < n && test(c)) {\n",
    replace: "    if (test(c)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^char_patterns.rs$"],
  },
  {
    name: "split-once-asks-every-char",
    breaks: "`split_once` and `rsplit_once` by a closure ask it of every `char`, front to back, where std's `find` and `rfind` stop at the first that holds, from the front or from the back (#1501)",
    file: "src/runtime/char_pattern.js",
    find: "  if (!last) {\n    let unit = 0;\n",
    replace: "  if (true) {\n    const found = $charMatches(s, test);\n    const at = last ? found[found.length - 1] : found[0];\n    return at === undefined ? undefined : split(at.unit, at.c);\n  }\n  if (!last) {\n    let unit = 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^str_split_once_by_a_closure_searches_as_std.rs$"],
  },
];
