// Mutations of src/lower/recognition/own_steps.rs (#182, #505).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lasts-its-own-not-through-skip",
    breaks:
      "`skip(n).last()` of the crate's iterator with its own `last` folds, not `nth(n - 1)` then its `last`",
    file: "src/lower/recognition/own_steps.rs",
    find: '            Some("Skip" | "Fuse" | "Peekable" | "Copied" | "Cloned") => 1,\n',
    replace:
      '            Some("Fuse" | "Peekable" | "Copied" | "Cloned") => 1,\n',
    tests: ["test/corpus.test.ts", "-t", "^own_last_and_nth_passed_on.rs$"],
  },
  {
    name: "lasts-its-own-not-through-chain",
    breaks:
      "a chain's `last` of the crate's iterators with their own `last` folds, not each side's `last`",
    file: "src/lower/recognition/own_steps.rs",
    find: '            Some("Skip" | "Fuse" | "Peekable" | "Copied" | "Cloned") => 1,\n            Some("Chain") => 2,\n',
    replace:
      '            Some("Skip" | "Fuse" | "Peekable" | "Copied" | "Cloned") => 1,\n            Some("Chain") => 0,\n',
    tests: ["test/corpus.test.ts", "-t", "^own_last_and_nth_passed_on.rs$"],
  },
  {
    name: "lasts-its-own-not-generic",
    breaks: "generic code's `last()` steps its iterator to its end, never asking the type's own `last` (#496)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            return !ty.is_ref() && self.own_iterator_method(ty, \"last\").is_some();\n        }\n        let ty::Adt(adt, args) = ty.kind() else {\n            return matches!(ty.kind(), ty::Param(_));\n        };\n        let sources = match",
    replace: "            return !ty.is_ref() && self.own_iterator_method(ty, \"last\").is_some();\n        }\n        let ty::Adt(adt, args) = ty.kind() else {\n            return false;\n        };\n        let sources = match",
    tests: ["test/corpus.test.ts", "-t", "^generic_own_last.rs$"],
  },
  {
    name: "counts-its-own-not-through-adapters",
    breaks: "std's compiled `Peekable` of the crate's iterator with its own `count` is given a dictionary with none, so its `count` steps it (#505)",
    file: "src/lower/recognition/own_steps.rs",
    find: "            Some(\"Skip\" | \"Fuse\" | \"Peekable\" | \"Enumerate\" | \"Copied\" | \"Cloned\") => 1,\n            Some(\"Chain\") => 2,\n            _ => return false,\n        };\n        args.types().take(sources).any(|source| self.counts_its_own(source))",
    replace: "            Some(\"Skip\" | \"Fuse\" | \"Peekable\" | \"Enumerate\" | \"Copied\" | \"Cloned\") => 1,\n            Some(\"Chain\") => 2,\n            _ => return false,\n        };\n        args.types().take(sources).any(|_| false)",
    tests: ["test/corpus.test.ts", "-t", "^own_count_through_adapters.rs$"],
  },
];
