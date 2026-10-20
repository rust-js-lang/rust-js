// Mutations of src/lower/analysis/clones.rs (ADR 0525).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clones-in-turn-unfollowed",
    breaks: "what a clone clones in turn, a derived `Clone`'s field's iterator, isn't seen to be cloned",
    file: "src/lower/analysis/clones.rs",
    find: "            todo.extend(clones_in_turn(tcx, ty));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "cloned-impl-unrevealed",
    breaks: "a clone of what an `impl Iterator + Clone` hides isn't seen to be of the iterator its function makes",
    file: "src/lower/analysis/clones.rs",
    find: "                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))).unwrap_or(ty)\n",
    replace: "                    ty\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "clone-bound-unseen",
    breaks: "a type a call's `Clone` bound is given isn't seen to be cloned, and its clone is refused or is the value itself",
    file: "src/lower/analysis/clones.rs",
    find: "            .filter(|p| is_clone(tcx, p.def_id()))\n            .map(|p| p.self_ty().skip_binder())\n            .collect()\n    })\n",
    replace: "            .filter(|p| false && is_clone(tcx, p.def_id()))\n            .map(|p| p.self_ty().skip_binder())\n            .collect()\n    })\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_counted.rs$"],
  },
];
