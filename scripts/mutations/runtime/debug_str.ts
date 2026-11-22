// Mutations of crates/rust-js-program/src/runtime/debug_str.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "debug-escapes-by-the-engine",
    breaks:
      "`{:?}` escapes what the JS engine's Unicode tables say, `'\\u{3f5cd}'` of a Unicode 17 engine, which Rust 1.100's Unicode 18 assigns and shows as it is (#1888)",
    file: "crates/rust-js-program/src/runtime/debug_str.js",
    find: "  if ($unicodeRegexp($unicodeDebugEscaped).test(c))",
    replace:
      "  if (/[\\p{Cc}\\p{Cf}\\p{Co}\\p{Cn}\\p{Zl}\\p{Zp}\\p{Grapheme_Extend}\\p{Default_Ignorable_Code_Point}]/u.test(c) || (c !== \" \" && /\\p{Zs}/u.test(c)))",
    tests: ["test/corpus.test.ts", "-t", "^unicode_answers_are_rusts.rs$"],
  },
];
