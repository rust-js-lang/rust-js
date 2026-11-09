// Mutations of src/lower/analysis/clones.rs (ADR 0525).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clones-in-turn-unfollowed",
    breaks:
      "what a clone clones in turn, a derived `Clone`'s field's iterator, isn't seen to be cloned",
    file: "src/lower/analysis/clones.rs",
    find: "            todo.extend(clones_in_turn(tcx, ty));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "cloned-impl-unrevealed",
    breaks:
      "a clone of what an `impl Iterator + Clone` hides isn't seen to be of the iterator its function makes",
    file: "src/lower/analysis/clones.rs",
    find: "                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))).unwrap_or(ty)\n",
    replace: "                    ty\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "clone-bound-unseen",
    breaks:
      "a type a call's `Clone` bound is given isn't seen to be cloned, and its clone is refused or is the value itself",
    file: "src/lower/analysis/clones.rs",
    find: "            .filter(|p| is_clone(tcx, p.def_id()))\n            .map(|p| p.self_ty().skip_binder())\n            .collect()\n    })\n",
    replace:
      "            .filter(|p| false && is_clone(tcx, p.def_id()))\n            .map(|p| p.self_ty().skip_binder())\n            .collect()\n    })\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_counted.rs$"],
  },
  {
    name: "library-given-out-clone-unseen",
    breaks:
      "a library's closure given out behind `impl FnMut() + Clone` is one it never clones, made with no clone, which its consumer's `tick.clone()` throws of (#1032)",
    file: "src/lower/analysis/clones.rs",
    find: "        todo.extend(given_out_clone(tcx, all_bodies));\n",
    replace: "        let _ = given_out_clone(tcx, all_bodies);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_singles_across_crates"],
  },
  {
    name: "generic-clone-uninstanced",
    breaks:
      "a clone in generic code of its own parameters' `Chain<J, J>` stays as rustc names it, though its callers are all seen, so another generic function's `Chain<I, I>` of `vec::IntoIter`s, whose bounds agree, is taken for it and its `next_back()` refused (#1119)",
    file: "src/lower/analysis/clones.rs",
    find: "            Some(instances) => instances.into_iter().collect(),\n",
    replace: "            Some(_) => vec![ty],\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_chains_cloned_by_their_instances.rs$"],
  },
  {
    name: "generic-cloned-apart-uninstanced",
    breaks:
      "a generic function's chain of its own parameters, all of whose callers are seen, is asked as it is, so another generic function's cloned `Chain<J, J>` of `Item = i32`, whose bounds agree, is taken for its `Chain<I, I>` of `vec::IntoIter`s, and its `next_back()` refused (#1119)",
    file: "src/lower/analysis/clones.rs",
    find: "            Some(made) => made.into_iter().any(is),\n",
    replace: "            Some(_) => is(ty),\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_chains_cloned_by_their_instances.rs$"],
  },
  {
    name: "generic-cloned-apart-unseen-callers-unasked",
    breaks:
      "a `pub` generic function's chain, which its callers unseen may give any type, is never cloned apart, though another's clone may be of its instance, so it's made not knowing its back where the refusal stood (#1119)",
    file: "src/lower/analysis/clones.rs",
    find: "            None => is(ty),\n",
    replace: "            None => false,\n",
    tests: ["test/diagnostics.test.ts", "-t", "pub generic chain stepped from its back"],
  },
];
