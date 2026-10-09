// Mutations of src/runtime/str_search.js (ADR 0323).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "split-inclusive-drops-match",
    breaks: "`split_inclusive` drops the match that ends each piece",
    file: "src/runtime/str_search.js",
    find: "    parts.push(s.slice(start, at + p.length));\n",
    replace: "    parts.push(s.slice(start, at));\n",
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
  {
    name: "rsplit-terminator-keeps-empty",
    breaks: "`rsplit_terminator` keeps the empty piece the end leaves",
    file: "src/runtime/str_search.js",
    find: "  if (parts[0] === \"\") parts.shift();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
  {
    name: "rmatch-indices-forward",
    breaks: "`rmatch_indices` searches from the start, so overlapping matches differ",
    file: "src/runtime/str_search.js",
    find: "  for (let at = s.lastIndexOf(p); at >= 0; at = at - p.length < 0 ? -1 : s.lastIndexOf(p, at - p.length)) {\n",
    replace: "  for (let at = s.indexOf(p); at >= 0; at = s.indexOf(p, at + p.length)) {\n",
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
  {
    name: "from-utf16-unchecked",
    breaks: "`from_utf16` of a lone surrogate is `Ok`",
    file: "src/runtime/str_search.js",
    find: "  return text.isWellFormed() ? { TAG: \"Ok\", _0: text } : { TAG: \"Err\", _0: undefined };\n",
    replace: "  return { TAG: \"Ok\", _0: text };\n",
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
  {
    name: "floor-char-boundary-after",
    breaks: "`floor_char_boundary` inside a `char` gives the boundary after it",
    file: "src/runtime/str_search.js",
    find: "    if (next > at) return bytes;\n",
    replace: "    if (next > at) return next;\n",
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
];
