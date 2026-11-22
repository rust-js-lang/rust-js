// Mutations of crates/rust-js-program/src/runtime/unicode.js, what a `char` is as Rust's tables
// say (#1888, ADR 1163).
import type { Mutation } from "../../mutations";

const file = "crates/rust-js-program/src/runtime/unicode.js";
const every = ["test/unicode.test.ts", "-t", "every char"];
const answers = ["test/corpus.test.ts", "-t", "^unicode_answers_are_rusts.rs$"];

export const mutations: Mutation[] = [
  {
    name: "alphabetic-by-the-engine",
    breaks: "`is_alphabetic()` is the JS engine's `\\p{Alphabetic}`, of its Unicode version, where Rust's is Unicode 18's",
    file,
    find: "  return $unicodeRegexp($unicodeAlphabetic).test(c);",
    replace: "  return /^\\p{Alphabetic}$/u.test(c);",
    tests: every,
  },
  {
    name: "unicode-set-boundaries-shifted",
    breaks: "a table's ranges end a `char` past their end, so the `char` after each is in it",
    file,
    find: "      else ranges += range(start, at - 1);",
    replace: "      else ranges += range(start, at);",
    tests: every,
  },
  {
    name: "unicode-map-run-unstrided",
    breaks: "a case map's run of `char`s each two apart, `Ā` `Ă` .., maps only its first",
    file,
    find: "      for (let i = 0; i < count; i++, at += stride) made.set(at, String.fromCodePoint(at + offset));",
    replace: "      for (let i = 0; i < count; i++) made.set(at, String.fromCodePoint(at + offset));",
    tests: every,
  },
  {
    name: "uppercase-by-the-engine",
    breaks: "`s.to_uppercase()` is the JS engine's `toUpperCase()`, which a Unicode 17 engine leaves U+0277 as",
    file,
    find: "  if (/^[\\0-\\x7f]*$/.test(s)) return s.toUpperCase();",
    replace: "  return s.toUpperCase();",
    tests: answers,
  },
  {
    name: "final-sigma-skips-nothing",
    breaks: "`to_lowercase` asks the `char` beside a `Σ` alone, so `ΑΣ'Β`'s, a word's end to it, is `ς`, where Rust skips the case-ignorable `'`",
    file,
    find: "if (!ignorable.test(chars[i])) return cased.test(chars[i]);",
    replace: "return cased.test(chars[i]);",
    tests: answers,
  },
  {
    name: "split-whitespace-js-s",
    breaks:
      "`split_whitespace()` splits by JS's `\\s`, at U+FEFF too, where Rust's whitespace is Unicode's `White_Space`",
    file,
    find: '  return s.split($unicodeRegexp($unicodeWhitespace, "#+")).filter((word) => word !== "");',
    replace: '  return s.split(/\\s+/).filter((word) => word !== "");',
    tests: answers,
  },
];
