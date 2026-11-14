// Mutations of src/lower/analysis/clones.rs (ADR 0525).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clone-of-any-type-may-be-a-closure",
    breaks:
      "a `pub` generic function's clone of whatever a consumer gives, `T`, may be any closure of the crate's, so an `Rc<dyn Fn()>` of one, compared, is counted, and the `Box<dyn Fn()>` holding it, handed a library whose `dyn Fn`s drop nothing, is refused: the react-in-rust port's react-reconciler (#1250)",
    file: "src/lower/analysis/clones.rs",
    find: "            .filter(|ty| !matches!(ty.kind(), ty::Param(_)))\n",
    replace: "",
    tests: ["test/cross-crate-dyn-fns.test.ts", "-t", "clone of any type"],
  },
  {
    name: "clones-in-turn-unfollowed",
    breaks:
      "what a clone clones in turn, a derived `Clone`'s field's iterator, isn't seen to be cloned",
    file: "src/lower/analysis/clones.rs",
    find: "            todo.extend(clones_in_turn(tcx, ty).into_iter().map(|part| (part, owner)));\n",
    replace: "            let _ = owner;\n",
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
    find: "        todo.extend(given_out_clone(tcx, all_bodies).into_iter().map(|ty| (ty, None)));\n",
    replace: "        let _ = given_out_clone(tcx, all_bodies);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_singles_across_crates"],
  },
  {
    name: "generic-clone-uninstanced",
    breaks:
      "a clone in generic code of its own parameters' `Chain<J, J>` stays as rustc names it, though its callers are all seen, so another generic function's `Chain<I, I>` of `vec::IntoIter`s, whose bounds agree, is taken for it and its `next_back()` refused (#1119)",
    file: "src/lower/analysis/clones.rs",
    find: "            Some(instances) => instances.into_iter().map(|ty| (ty, None)).collect(),\n",
    replace: "            Some(_) => vec![(ty, None)],\n",
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
  {
    name: "given-out-clone-unrevealed-in-turn",
    breaks:
      "what a library's `impl Iterator + Clone` hides is left as another function's `impl Iterator + Clone`, so the closure that one hides is never decided, made a plain function, and its consumer's clone throws (#1202)",
    file: "src/lower/analysis/clones.rs",
    find: "                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(hidden)))\n                        .unwrap_or(hidden),\n",
    replace: "                    hidden,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_closures_cloned_as_made"],
  },
  {
    name: "compiled-behind-an-impl-cloned-apart",
    breaks: "std's struct a function makes behind an `impl Iterator + Clone` is taken for an adapter the crate clones apart, so a clone of a filter over it clones it as one, `f[$cloneKey] is not a function` (#1241)",
    file: "src/lower/analysis/clones.rs",
    find: "            true => compiled_seen.insert(ty),\n            false => cloned.types.insert(ty),\n",
    replace: "            _ => cloned.types.insert(ty),\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_of_an_adapter_over_a_returned_impl.rs$"],
  },
  {
    name: "compiled-struct-parts-unseen",
    breaks: "std's struct a library's function gives behind an `impl .. + Clone` is left out of what the crate clones with its parts, so its consumer's clone of it, by its compiled `Clone`, finds its closure never seen cloned and is refused (#1242)",
    file: "src/lower/analysis/clones.rs",
    find: "            true => compiled_seen.insert(ty),\n",
    replace: "            true => false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_closures_cloned_as_made"],
  },
  {
    name: "library-closure-clones-unseen",
    breaks:
      "a library's closure boxed cloned and as it is, compiled by a consumer that reads a `dyn Fn`'s address, of MIR alone, isn't among what the crate clones, so it's taken to be made with each box, the two one function, `addr_eq` `true` (#1053)",
    file: "src/lower/analysis/clones.rs",
    find: "            .flat_map(|body| mir_called_clones(tcx, body))\n",
    replace: "            .flat_map(|_| Vec::<Ty<'tcx>>::new())\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_allocations_of_one_function"],
  },
  {
    name: "cloned-owners-unrecorded",
    breaks: "a type cloned as it is of its function's own parameters keeps no owner, so no bounds of it are asked, and another generic function's `map(Some)` is cloned apart and refused (#1318)",
    file: "src/lower/analysis/clones.rs",
    find: "            cloned.owners.entry(ty).or_default().insert(owner);\n",
    replace: "            let _ = owner;\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_map_past_a_take_beside_a_cloned_one.rs$"],
  },
];
