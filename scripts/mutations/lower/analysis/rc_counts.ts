// Mutations of src/lower/analysis/rc_counts.rs (ADR 0320).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rc-generic-uncounted",
    breaks: "a generic function's `Rc<T>` isn't counted where its caller's is",
    file: "src/lower/analysis/rc_counts.rs",
    find: "    counted.all = counted.types.iter().any(|t| t.has_non_region_param()) || !counted.types.is_empty() && generic_rc;\n",
    replace: "    counted.all = false && generic_rc;\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "rc-destructor-uncounted",
    breaks: "an `Rc` of a value with a destructor isn't counted, and is refused",
    file: "src/lower/analysis/rc_counts.rs",
    find: "            .filter(|&&pointee| !counted.types.contains(&pointee) && runs(&counted, pointee))\n",
    replace: "            .filter(|&&pointee| !counted.types.contains(&pointee) && false && runs(&counted, pointee))\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_destructors.rs$"],
  },
  {
    name: "rc-destructor-one-round",
    breaks: "an `Rc` of a value that holds a counted `Rc` isn't counted, as its drop runs only once that one is",
    file: "src/lower/analysis/rc_counts.rs",
    find: "        counted.types.extend(dropping);\n    }\n",
    replace: "        counted.types.extend(dropping);\n        break;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_destructors.rs$"],
  },
];
