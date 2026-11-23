// Mutations of crates/rust-js-program/src/runtime/unzip.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unzip-swapped",
    breaks: "`unzip()` gives the second parts first",
    file: "crates/rust-js-program/src/runtime/unzip.js",
    find: "    a.push(x);\n",
    replace: "    a.push(y);\n",
    tests: ["test/corpus.test.ts","-t","iterator_sources"],
  },
  {
    name: "unzip-steps",
    breaks: "`unzip` of the crate's iterator steps it by `next`, not by its own `fold`",
    file: "crates/rust-js-program/src/runtime/unzip.js",
    find: "    () => (typeof pairs.fold === \"function\" ? pairs.fold(undefined, (_, pair) => put(pair)) : $eachOf(pairs, put)),\n",
    replace: "    () => $eachOf(pairs, put),\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "unzip-collected-leaked",
    breaks: "`unzip` whose source's step panics never drops what it unzipped, which std's `(A, B)` drops as it unwinds (#1150)",
    file: "crates/rust-js-program/src/runtime/unzip.js",
    find: "    dropA && (() => $dropItems(a, dropA)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^consumers_keep_what_they_built_on_a_panic.rs$"],
  },
  {
    name: "unzip-reserves-nothing",
    breaks: "`unzip()` into `Vec`s reserves no room for its hint's lower bound, so a `Vec`'s `into_iter()` of 10 gives sides of capacity 16 where std's `default_extend` reserves 10 (#2405)",
    file: "crates/rust-js-program/src/runtime/unzip.js",
    find: "      if (kept(sizeA)) $reserve(a, lower, sizeA);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unzip_sides_grow_as_std_extends_them.rs$"],
  },
  {
    name: "unzip-growth-unkept",
    breaks: "`unzip()` into `Vec`s keeps no growth as it pushes, so a `filter`'s 6 give a capacity of 6 where std's `push` grows it 4, 8 (#2405)",
    file: "crates/rust-js-program/src/runtime/unzip.js",
    find: "    if (kept(sizeA)) $grew(a, sizeA, len);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unzip_sides_grow_as_std_extends_them.rs$"],
  },
  {
    name: "unzip-hint-unasked",
    breaks: "`unzip()` never asks what it steps its `size_hint()`, which std's `default_extend` asks once, so the crate's own that prints never does (#2405)",
    file: "crates/rust-js-program/src/runtime/unzip.js",
    find: "  const [asked] = Array.isArray(pairs) ? [pairs.length] : $sizeHint(pairs);\n",
    replace: "  const [asked] = Array.isArray(pairs) ? [pairs.length] : [0];\n",
    tests: ["test/corpus.test.ts", "-t", "^unzip_asks_its_hint.rs$"],
  },
  {
    name: "unzip-std-lower-unread",
    breaks: "`unzip()` of a `filter` JS made an array at once reserves its length, so 6 kept give a capacity of 6 where std's lower bound, 0, gives 8 (#2405)",
    file: "crates/rust-js-program/src/runtime/unzip.js",
    find: "  const lower = stdLower ?? asked;\n",
    replace: "  const lower = asked;\n",
    tests: ["test/corpus.test.ts", "-t", "^unzip_sides_grow_as_std_extends_them.rs$"],
  },
  {
    name: "unzip-trusted-unreserved",
    breaks: "`unzip()` of a `TrustedLen` keeps no capacity for its sides, so a `map` by a named function, whose JS form hints none, gives 10 a capacity of 10 by its length alone and two `u8`s 2 where std reserves 8 (#2405)",
    file: "crates/rust-js-program/src/runtime/unzip.js",
    find: "    if (sizeB !== undefined) $reservedFor(b, sizeB);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unzip_sides_grow_as_std_extends_them.rs$"],
  },
];
