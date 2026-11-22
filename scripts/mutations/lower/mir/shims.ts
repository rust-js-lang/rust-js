// Mutations of crates/rust-js-mir/src/shims.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reified-value-not-a-pointer",
    breaks: "a `#[track_caller]` function made a `fn` pointer isn't known to be one, so it's given its caller's location, not where it's defined (#2335)",
    file: "crates/rust-js-mir/src/shims.rs",
    find: "                    Way::Called(Through::Pointer)\n",
    replace: "                    Way::Unknown\n",
    tests: ["test/corpus.test.ts", "-t", "^std_function_pointer_panic.rs$"],
  },
  {
    name: "value-in-a-local-unfollowed",
    breaks: "a function value put in a local isn't followed to the local's uses, so `let f = Option::unwrap; v.map(f)` names its caller's line (#2335)",
    file: "crates/rust-js-mir/src/shims.rs",
    find: "Some(local) => local_through(tcx, body, local, depth + 1),",
    replace: "Some(_) => Way::Unknown,",
    tests: ["test/corpus.test.ts", "-t", "^std_function_value_in_a_local_panic.rs$"],
  },
  {
    name: "mention-taken-for-a-use",
    breaks: "a local's mention, its `let`'s, that reads nothing, is taken for a use, so a function value put in a local isn't decided (#2335)",
    file: "crates/rust-js-mir/src/shims.rs",
    find: "            | PlaceContext::NonMutatingUse(Read::Inspect)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_function_value_in_a_local_panic.rs$"],
  },
  {
    name: "fn-once-shim-its-output",
    breaks: "`FnOnce`'s shim taken for its first associated item, its `Output`, whose line is named, not `call_once`'s (#2335)",
    file: "crates/rust-js-mir/src/shims.rs",
    find: "            .find(|item| item.is_fn())\n",
    replace: "            .next()\n",
    tests: ["test/corpus.test.ts", "-t", "^std_function_value_panic_once.rs$"],
  },
];
