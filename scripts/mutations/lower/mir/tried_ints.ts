// Mutations of src/lower/mir/tried_ints.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tried-int-returned",
    breaks:
      "a `try_from`'s `Result` given back whole is taken for one only tested, `return result;` of nothing made (#366)",
    file: "src/lower/mir/tried_ints.rs",
    find: "tried && destination.projection.is_empty() && local != mir::RETURN_PLACE && calls(local) == 1",
    replace: "tried && destination.projection.is_empty() && calls(local) == 1",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
  },
  {
    name: "tried-int-let-mention-unknown",
    breaks:
      "a `let Ok(n) = u8::try_from(i) else { .. }` is `$tryFromInt`, as the `let`'s mention of its `Result` was taken for a read of it whole (#366)",
    file: "src/lower/mir/tried_ints.rs",
    find: "NonMutatingUseContext::Inspect | NonMutatingUseContext::PlaceMention",
    replace: "NonMutatingUseContext::Inspect",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
    snapshots: true,
  },
  {
    name: "tried-int-error-read",
    breaks:
      'a `try_from` whose error is read, `Err(e) => format!("{e}")`, is taken for one only tested, its error never made (#366)',
    file: "src/lower/mir/tried_ints.rs",
    find: "                    ) => variant.as_u32() == 0,\n",
    replace: "                    ) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
  },
  {
    name: "tried-int-not-held",
    breaks:
      "a `try_from` of what may change, `counter.at`, is read at its test and again where its `Ok` is, not held once, `const value = counter.at` (#366)",
    file: "src/lower/mir/tried_ints.rs",
    find: "        n.visit_vars(&mut |name| plain &= self.plain_local(name));\n",
    replace:
      "        n.visit_vars(&mut |name| plain &= self.plain_local(name) || true);\n",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
    snapshots: true,
  },
];
