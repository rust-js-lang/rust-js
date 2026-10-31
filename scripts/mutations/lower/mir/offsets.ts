// Mutations of src/lower/mir/offsets.rs (ADR 0093, ADR 0642).
import type { Mutation } from "../../../mutations";

const places = ["test/lowering-string-places.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "offsets-none-in-units",
    breaks:
      "no local is a place in units, but its find is still `indexOf`, so a found place is tested `!= null` and `-1` is found",
    file: "src/lower/mir/offsets.rs",
    find: "        self.units.contains(&local)\n",
    replace: "        let _ = local;\n        false\n",
    tests: [...places, "-t", "owner stack"],
  },
  {
    name: "offsets-new-string-old-places",
    breaks:
      "a string a variable is given keeps the places of the one it held, so `&s[at..]` after `s = &s[1..]` slices by units",
    file: "src/lower/mir/offsets.rs",
    find: "        let made = || text(destination.local).then_some(Fact::Text(Text::At(at)));",
    replace:
      "        let made = || text(destination.local).then_some(Fact::Text(Text::Entry(mir::Local::from_u32(1))));",
    tests: [...places, "-t", "held before"],
  },
  {
    name: "offsets-sliced-at-another-strings-places",
    breaks:
      "a place found in one string slices another in units, `other.slice(0, at)`",
    file: "src/lower/mir/offsets.rs",
    find: "                Some(&Fact::Bounds { text: bounds, .. }) if known.within(bounds, text)),",
    replace:
      "                Some(&Fact::Bounds { text: bounds, .. }) if known.within(bounds, bounds)),",
    tests: [...places, "-t", "keeps counting bytes"],
  },
  {
    name: "offsets-suffix-a-prefix",
    breaks:
      "a slice from a place, a suffix, is taken for a prefix, so a place found in it slices the string in units",
    file: "src/lower/mir/offsets.rs",
    find: "                return (made(), of(0).filter(|_| prefix));",
    replace: "                return (made(), of(0));",
    tests: [...places, "-t", "found in"],
  },
  {
    name: "offsets-step-by-any-constant",
    breaks:
      "a place stepped by a constant that isn't what was found, `at + 3` past an `'a'`, is a place, written `at + 1`",
    file: "src/lower/mir/offsets.rs",
    find: "Fact::LiteralLength { bytes: by, .. }) if bytes == by => {",
    replace:
      "Fact::LiteralLength { bytes: by, .. }) if bytes == by || by > 0 => {",
    tests: [...places, "-t", "keeps counting bytes"],
  },
  {
    name: "offsets-step-by-another-strings-length",
    breaks:
      "a place stepped by another string's length than what was found is a place, `at + q.length`",
    file: "src/lower/mir/offsets.rs",
    find: "                        (Pattern::Held(of), Fact::Length(length)) if of == length => Some(Fact::Place(text)),",
    replace:
      "                        (Pattern::Held(_), Fact::Length(_)) => Some(Fact::Place(text)),",
    tests: [...places, "-t", "found in"],
  },
  {
    name: "offsets-range-of-two-places",
    breaks:
      "a range of two places found apart is JS's `slice`, which gives `\"\"` where Rust's panics, its end before its start",
    file: "src/lower/mir/offsets.rs",
    find: "                        (start, Fact::Length(end)) if known.common(start.place()?, Some(end)) == Some(Some(end)) => {",
    replace:
      "                        (start, end) if let Some(Some(end)) = end.place() && known.common(start.place()?, Some(end)) == Some(Some(end)) => {",
    tests: [...places, "-t", "found in"],
  },
  {
    name: "offsets-units-settled-at-once",
    breaks:
      "one pass settles which locals are in units, so a find whose place is compared with a number after is `indexOf` still",
    file: "src/lower/mir/offsets.rs",
    find: "            if !changed {\n                break;\n            }",
    replace:
      "            if changed || !changed {\n                break;\n            }",
    tests: [...places, "-t", "keeps counting bytes"],
  },
  {
    name: "offsets-literal-pattern-bytes",
    breaks:
      "a literal pattern's length in units is its bytes', so `strip_suffix(\"🦀\")` cuts four units off",
    file: "src/lower/mir/offsets.rs",
    find: "                        units: literal.encode_utf16().count(),",
    replace: "                        units: literal.len(),",
    tests: [...places, "-t", "strip tested"],
  },
  {
    name: "offsets-cut-reread-by-another-strip",
    breaks:
      "a cut a strip tests and cuts again is read in place, so its string is held first, `const text$1 = text.slice(1)`",
    file: "src/lower/mir/offsets.rs",
    find: "                    && !strips_again(body, *at, given, &strips)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "const_blocks_and_let_guards"],
    snapshots: true,
  },
];
