// Mutations of src/lower/analysis/hint_asked.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "hint-asked-unmarked",
    breaks: "a `filter` a caller makes and gives generic code that asks its hint is JS's own `values().filter(p)`, whose hint is `(0, None)`, where std's is `(0, Some(4))` (#1236)",
    file: "src/lower/analysis/hint_asked.rs",
    find: "                mark_hint_knowing(tcx, recognition, instance, (&mut asked.knowing, &mut asked.hinted));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_adapter_given_to_generic_code_that_asks_its_hint\\.rs$"],
  },
  {
    name: "library-callee-hint-unmarked",
    breaks: "a `filter` given a library's generic function that asks its hint is JS's own, `(0, None)`, as the crate can't see the library's body (#1236)",
    file: "src/lower/analysis/hint_asked.rs",
    find: "                    mark_hint_knowing(\n                        tcx,\n                        recognition,\n                        given.peel_refs(),\n                        (&mut asked.knowing, &mut asked.hinted),\n                    );\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_hint_asked_of_what_its_given$"],
  },
];
