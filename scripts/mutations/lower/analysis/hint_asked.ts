// Mutations of src/lower/analysis/hint_asked.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "hint-asked-unmarked",
    breaks: "a `filter` a caller makes and gives generic code that asks its hint is JS's own `values().filter(p)`, whose hint is `(0, None)`, where std's is `(0, Some(4))` (#1236)",
    file: "src/lower/analysis/hint_asked.rs",
    find: "                of_items |= mark_hint_knowing(\n                    tcx,\n                    recognition,\n                    instance,\n                    (&mut asked.knowing, &mut asked.hinted.types),\n                );\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_adapter_given_to_generic_code_that_asks_its_hint\\.rs$"],
  },
  {
    name: "library-callee-hint-unmarked",
    breaks: "a `filter` given a library's generic function that asks its hint is JS's own, `(0, None)`, as the crate can't see the library's body (#1236)",
    file: "src/lower/analysis/hint_asked.rs",
    find: "                    _ = mark_hint_knowing(\n                        tcx,\n                        recognition,\n                        given.peel_refs(),\n                        (&mut asked.knowing, &mut asked.hinted.types),\n                    );\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_hint_asked_of_what_its_given$"],
  },
  {
    name: "made-in-generic-code-unmarked",
    breaks: "a `map` or a `take` generic code makes of its own type parameter's iterator and gives code that asks its hint is JS's own, `(0, None)`, where std's is `(5, Some(5))` (#1240)",
    file: "src/lower/analysis/hint_asked.rs",
    find: "                    mark_made_in(tcx, maker, made, &mut asked.hinted);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_made_in_generic_code_asked_its_hint\\.rs$"],
  },
  {
    name: "unseen-callers-made-unmarked",
    breaks: "a library's generic function's `map` of what a consumer gives it, given on where its hint is asked, is JS's own, `(0, None)`, as the library sees none of its callers (#1240)",
    file: "src/lower/analysis/hint_asked.rs",
    find: "crate::lower::instances::instances_of(tcx, &callers, of, owner).is_none()",
    replace: "false",
    tests: ["test/crates.test.ts", "-t", "two crates: library_adapter_made_in_generic_code_asked_its_hint$"],
  },
  {
    name: "generic-growth-lost-unseen",
    breaks:
      "a `pub` generic function's `extend` of a `Vec`, whose callers the crate can't see, is lost: its `capacity()` throws where std's is 32 (#1180's (3))",
    file: "src/lower/analysis/hint_asked.rs",
    find: "                None => _ = asked.hinted.by_value.insert((owner, iter_ty)),\n",
    replace: "                None => _ = asked.hinted.lost.insert((owner, iter_ty)),\n",
    tests: ["test/lowering-collections.test.ts", "-t", "hint JS lost"],
  },
  {
    name: "generic-growth-instances-unasked",
    breaks:
      "generic code's `extend` of a `Vec` by what its callers give is never lost: of `chars().skip(5)` given it, its capacity 64 where std's is 60, silently (#1180's (3))",
    file: "src/lower/analysis/hint_asked.rs",
    find: "if (instances.into_iter()).any(|instance| !recognition.given_growth_kept(instance)) {",
    replace: "if false && (instances.into_iter()).any(|instance| !recognition.given_growth_kept(instance)) {",
    tests: ["test/lowering-collections.test.ts", "-t", "hint JS lost"],
  },
  {
    name: "generic-growth-unseen-not-by-value",
    breaks:
      "a `pub` generic function's `extend` of a `Vec`, whose callers the crate can't see, isn't asked of each value it's given: of `chars().skip(5)`, a capacity of 64 where std's is 44, silently (#1251)",
    file: "src/lower/analysis/hint_asked.rs",
    find: "                None => _ = asked.hinted.by_value.insert((owner, iter_ty)),\n",
    replace: "                None => {}\n",
    tests: ["test/lowering-collections.test.ts", "-t", "hint JS lost"],
  },
];
