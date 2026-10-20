// Mutations of src/lower/analysis/fn_dyns.rs (ADR 0521).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fn-dyn-drops-unseen",
    breaks: "a function holding a value with a destructor made a `dyn Fn` leaves the `dyn Fn` dropping nothing",
    file: "src/lower/analysis/fn_dyns.rs",
    find: "        .any(|from| drops(from) || from.has_non_region_param() && closure_drops())\n",
    replace: "        .any(|from| from.has_non_region_param() && closure_drops())\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
  {
    name: "fn-dyn-generic-unseen",
    breaks: "a type parameter made a `dyn Fn`, given a closure holding a value with a destructor, leaves the `dyn Fn` dropping nothing",
    file: "src/lower/analysis/fn_dyns.rs",
    find: "        .any(|from| drops(from) || from.has_non_region_param() && closure_drops())\n",
    replace: "        .any(|from| drops(from))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_generic_drops.rs$"],
  },
  {
    name: "fn-dyn-closure-drops-unseen",
    breaks: "a closure holding a value with a destructor, given to a type parameter made a `dyn Fn`, isn't seen to drop",
    file: "src/lower/analysis/fn_dyns.rs",
    find: "                    drops(ty) || params_may_drop && ty.has_non_region_param()\n",
    replace: "                    params_may_drop && ty.has_non_region_param()\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_generic_drops.rs$"],
  },
  {
    name: "fn-dyn-closure-uncarried",
    breaks: "a closure made a `dyn Fn` doesn't carry its drop, and `Box::new` of it is refused",
    file: "src/lower/analysis/fn_dyns.rs",
    find: "        self.closures.contains(&closure) || given_back(tcx, closure)\n",
    replace: "        given_back(tcx, closure)\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
];
