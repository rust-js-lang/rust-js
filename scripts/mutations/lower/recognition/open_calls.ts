// Mutations of src/lower/recognition/open_calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "own-reader-names-no-issue",
    breaks: "a call of a crate's reader's method is refused naming no issue, where #1297 covers it",
    file: "src/lower/recognition/open_calls.rs",
    find: "        return Some(OpenCall::OwnReader);\n",
    replace: "        return None;\n",
    tests: ["test/diagnostics.test.ts", "-t", "a read of a crate's reader"],
  },
  {
    name: "std-hash-evidence-unnamed",
    breaks: "a std type's missing `Hash`, a `Duration`'s, names no issue, where #1447 covers it",
    file: "src/lower/recognition/open_calls.rs",
    find: "        return Some(OpenCall::StdHash);\n",
    replace: "        return None;\n",
    tests: ["test/diagnostics.test.ts", "-t", "a set of a Duration beside"],
  },
  {
    name: "newtype-sum-unnamed",
    breaks: "a `Wrapping`'s `sum()` names no issue, where #1445 covers it",
    file: "src/lower/recognition/open_calls.rs",
    find: "    (sum && newtype).then_some(OpenCall::NewtypeSum)\n",
    replace: "    (sum && newtype && false).then_some(OpenCall::NewtypeSum)\n",
    tests: ["test/diagnostics.test.ts", "-t", "a sum of Wrappings"],
  },
  {
    name: "rc-pin-unnamed",
    breaks: "`Rc::pin` names no issue, where #1448 covers it",
    file: "src/lower/recognition/open_calls.rs",
    find: "        return Some(OpenCall::PinRead);\n",
    replace: "        return None;\n",
    tests: ["test/diagnostics.test.ts", "-t", "an Rc's pin"],
  },
];
