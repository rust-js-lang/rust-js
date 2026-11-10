// Mutations of src/lower/instances/needed.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "instance-drop-ignored",
    breaks:
      "a consumer's call that gives a library's generic function a value with a destructor its JS takes no drop for calls the library's, so a panic in it drops nothing, where Rust drops `Loud(2)` (ADR 0914)",
    file: "src/lower/instances/needed.rs",
    find: "        (drops && !takes(index, ParamFact::Drop) && body_may_drop(calls, (callee, index), may_drop))\n",
    replace:
      "        (false && body_may_drop(calls, (callee, index), may_drop))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "instance-copy-ignored",
    breaks:
      "a consumer's call that gives a library's `T: Copy` function a `Point` it changes in place calls the library's, which copies nothing, so the two cells it made hold one object (ADR 0914)",
    file: "src/lower/instances/needed.rs",
    find: "            || (copies && copied.contains(&index) && !takes(index, ParamFact::Copy))\n",
    replace: "            || (false && copied.contains(&index))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_given"],
  },
  {
    name: "instance-private-callee-imported",
    breaks:
      "a library's private function an instance calls isn't compiled with it, and is imported from a module that doesn't export it (ADR 0914)",
    file: "src/lower/instances/needed.rs",
    find: "            let private = instance && foreign.item(callee).is_none();\n",
    replace: "            let private = false;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "instance-private-state-unrefused",
    breaks:
      "an instance that reads its library's private thread-local isn't refused, and reads one of its own, or none (ADR 0914)",
    file: "src/lower/instances/needed.rs",
    find: "            refuse_private_state(&calls, body.def_id, &mir.body);\n",
    replace: "            let _ = refuse_private_state;\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "reads its library's private state"],
  },
  {
    name: "instance-copy-of-a-given-param-ignored",
    breaks:
      "a consumer's generic function given a copy of its `T` gives a library's function its `T` with none, as if its own copy (ADR 0914)",
    file: "src/lower/instances/needed.rs",
    find: "            _ => !crate::lower::analysis::copies_itself(tcx, given),\n",
    replace: "            _ => false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_given"],
  },
  {
    name: "cleanup-always-unwound",
    breaks:
      "a library's function's drop where a binding that can't throw unwinds is taken for one, and a consumer compiles its own `count` that drops nothing (ADRs 0301, 0914)",
    file: "src/lower/instances/needed.rs",
    find: "        _ if data.is_cleanup && !unwound.contains(&block) => false,\n",
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "can't throw, read before a move"],
  },
  {
    name: "copied-argument-not-given",
    breaks:
      "a value optimized MIR copies where it's last used, `pick::<(), T>(const (), copy _1)`, isn't taken for given, so `discard` drops nothing of a consumer's `Loud` (ADR 0914)",
    file: "src/lower/instances/needed.rs",
    find: "                    mir::Operand::Move(place) | mir::Operand::Copy(place) => lent(place.ty(&body.local_decls, tcx).ty),\n",
    replace: "                    mir::Operand::Move(place) => lent(place.ty(&body.local_decls, tcx).ty),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: drops_by_need"],
  },
  {
    name: "closure-call-may-drop",
    breaks:
      "a value a library's function gives a closure it's given is taken for one it may drop, and a consumer compiles `apply` and `keep_by`, which drop nothing (ADR 0914)",
    file: "src/lower/instances/needed.rs",
    find: "                    {\n                        false\n                    }\n                    None => false,\n",
    replace:
      "                    {\n                        true\n                    }\n                    None => true,\n",
    tests: ["test/crates.test.ts", "-t", "takes the drops its body uses"],
  },
  {
    name: "instance-clones-unasked",
    breaks:
      "a consumer that gives a library's generic function a value that isn't its own clone, where its export takes no `TClone`, calls the export, whose clone is the value itself, and its copy changes what it was given (ADR 0914, amended)",
    file: "src/lower/instances/needed.rs",
    find: "            || (clones && cloned.contains(&index) && !takes(index, ParamFact::Clone))\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "compiles its own of the function",
    ],
  },
  {
    name: "instance-gives-nullish-unasked",
    breaks:
      "a consumer that gives a library's unboxed type parameter a `()` calls the export, whose `Some(())` is `undefined`, `None` (ADR 0051, amended)",
    file: "src/lower/instances/needed.rs",
    find: "            || (nullish\n                && counted(index)\n",
    replace: "            || (false\n                && counted(index)\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "unboxed function"],
  },
  {
    name: "instance-binding-static-private",
    breaks:
      "a library's function its consumer compiles is refused where it reads a binding's static, a global JS has, as React DOM's call of the reconciler's runWithFiberInDEV reading shared's `__DEV__` was (ADR 0914, amended, gap rd75)",
    file: "src/lower/instances/needed.rs",
    find: "if state && !binding && !def.is_local()",
    replace: "if state && !def.is_local()",
    tests: ["test/crates-compiled-here.test.ts", "-t", "private state"],
  },
  {
    name: "instance-relative-static-read",
    breaks:
      "a library's function its consumer compiles reads a binding's static its library binds to a module by a path from its own file, which the consumer's file doesn't name (ADR 0914, amended)",
    file: "src/lower/instances/needed.rs",
    find: ".chain(statics)",
    replace: ".chain(statics.take(0))",
    tests: ["test/crates-compiled-here.test.ts", "-t", "private state"],
  },
  {
    name: "instance-unit-given-copied",
    breaks:
      "a library's generic function given `()`s, which may look like `None`, is compiled by its consumer, where it makes no `Some` of them and its export serves, as React DOM copied the reconciler's runWithFiberInDEV (ADR 0914, amended, gap rd75b)",
    file: "src/lower/instances/needed.rs",
    find: "                && body_may_box(calls, (callee, index), &mut IndexSet::default()))",
    replace: "                && true)",
    tests: ["test/crates.test.ts", "-t", "given what may look like None"],
  },
  {
    name: "instance-box-unseen",
    breaks:
      "a library's generic function that makes a `Some` of what it's given, `wrapped(())`, is called as its export, which boxes none, so its `Some(())` is `None` (ADR 0051, amended)",
    file: "src/lower/instances/needed.rs",
    find: "    if body.local_decls.iter().any(|decl| boxes(decl.ty)) {\n",
    replace:
      "    if false && body.local_decls.iter().any(|decl| boxes(decl.ty)) {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: generic_given_unit"],
  },
  {
    name: "instance-sets-library-variable",
    breaks:
      "a library's function its consumer compiles sets its library's module variable, an import, which JS can't assign: a TypeError where Rust runs (ADR 0270, ADR 0914, amended)",
    file: "src/lower/instances/needed.rs",
    find: "if sets && let Some(&key)",
    replace: "if false && sets && let Some(&key)",
    tests: ["test/crates-compiled-here.test.ts", "-t", "private state"],
  },
  {
    name: "through-library-compiled",
    breaks:
      "a library's namespace's method called through a value is one called by its name, compiled into the consumer where its call needs a copy (ADR 0914), and refused there for its library's private state, not for the copy JS's value can't be (rd311)",
    file: "src/lower/instances/needed.rs",
    find: "                through.then_some((callee, generic, *fn_span, destination.local))\n",
    replace: "                through.then_some((callee, generic, *fn_span, destination.local)).filter(|_| false)\n",
    tests: ["test/namespaces.test.ts", "-t", "through a value whose call needs a copy is refused"],
  },
  {
    name: "through-library-unrefused",
    breaks:
      "a library's namespace's method through a value whose call gives what may look like `None` and isn't flattened is called, its `Some(None)` read as `None` (rd311)",
    file: "src/lower/instances/needed.rs",
    find: "                crate::lower::classes::refuse_through(tcx, callee, span, !other);\n",
    replace: "                let _ = (span, other);\n",
    tests: ["test/namespaces.test.ts", "-t", "refused unless its result is flattened"],
  },
  {
    name: "through-library-flattened-refused",
    breaks:
      "a library's namespace's method through a value whose result only `.flatten()` reads is refused (rd311)",
    file: "src/lower/instances/needed.rs",
    find: "            if gives_more(&calls, given, &|index| Some(index) != flattened, &mut may_drop) {\n",
    replace: "            if gives_more(&calls, given, &|_| true, &mut may_drop) {\n",
    tests: ["test/namespaces.test.ts", "-t", "exact where its result is flattened"],
  },
  {
    name: "flattened-by-name-compiled-here",
    breaks:
      "a namespace's method by its name whose result is flattened is compiled in the consumer, which reads its library's private state, refused, react-test-renderer's `unstable_flushSync` (rd324)",
    file: "src/lower/instances/needed.rs",
    find: "            let flattened = crate::lower::analysis::flattened_by_name(&calls.recognition, &mir.body, (callee, args));\n",
    replace: "            let flattened: Option<u32> = None;\n",
    tests: ["test/namespaces.test.ts", "-t", "by its name is its library"],
  },
  {
    name: "writer-refusal-unnamed",
    breaks:
      "a writer compiled here for the options it's given, refused where it reads its library's private state, says it's compiled for a copy or a drop, not what it's given (#1026)",
    file: "src/lower/instances/needed.rs",
    find: "    match foreign.writes_more(root) {\n",
    replace: "    match foreign.writes_more(root).filter(|_| false) {\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "refused where it reads its library's private state"],
  },
  {
    name: "items-given-drops-unasked",
    breaks:
      "a call giving a library's function an iterator of no destructor whose items have one, `second(Gen(0))`, gives it nothing more, so its export, which takes no drop for `I::Item`, never drops the item it passes over (#1031)",
    file: "src/lower/instances/needed.rs",
    find: "    }) || items_given_drops(calls, (caller, callee, args), &takes)\n",
    replace: "    }) || (false && items_given_drops(calls, (caller, callee, args), &takes))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
  {
    name: "owned-param-given-no-drop",
    breaks:
      "a type that owns the caller's type parameter, a closure holding a `Vec<K>`, given a library's function, gives it nothing to drop where the caller is given a drop for `K`, so `boxed`'s `dyn Fn` drops nothing (#1031)",
    file: "src/lower/instances/needed.rs",
    find: "            _ if given.has_param() => crate::lower::drops::owned_params(tcx, given).iter().any(caller_drops),\n",
    replace: "            _ if given.has_param() => false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
  {
    name: "drop-in-place-drops-nothing",
    breaks:
      "a library's body's `ptr::drop_in_place::<T>(p)` drops no `T` to `body_may_drop`, its `*mut T` owning none, so `Slots::truncate` is its export, which drops nothing (#1031)",
    file: "src/lower/instances/needed.rs",
    find: "                    return callee_args.types().any(holds);\n",
    replace: "                    return false;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
  {
    name: "item-slot-callee-drops-nothing",
    breaks:
      "a library's body that gives its `M: Iterator<Item = T>` to a function of its library's that drops items, `second(made)`, drops no `T` to `body_may_drop`, so `made_second` is its export, which drops nothing (#1031)",
    file: "src/lower/instances/needed.rs",
    find: "                    && compiled_here(calls.foreign, tcx, callee)\n                    && crate::lower::traits::drop_items(tcx, calls.foreign, callee)\n",
    replace: "                    && compiled_here(calls.foreign, tcx, callee)\n                    && false\n                    && crate::lower::traits::drop_items(tcx, calls.foreign, callee)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
  {
    name: "lent-mut-drops-nothing",
    breaks:
      "a library's body that lends a `&mut Slot<T>` to what may write its `T` again, `slot.renew()`, drops no `T` to `body_may_drop`, so `renewed` is its export, which drops the old one nowhere (#1032)",
    file: "src/lower/instances/needed.rs",
    find: "                    holds(ty) || matches!(*ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if holds(inner))\n",
    replace: "                    holds(ty)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_singles_across_crates"],
  },
  {
    name: "callee-given-holder-drops-nothing",
    breaks:
      "a library's body that gives what holds its `T`, not `T` itself, to a function of its library's drops no `T` to `body_may_drop` (#1032)",
    file: "src/lower/instances/needed.rs",
    find: "                            (callee_args.get(at as usize).and_then(|arg| arg.as_type())).is_some_and(holds)\n                                && body_may_drop(calls, (callee, at), seen)\n",
    replace: "                            (callee_args.get(at as usize).and_then(|arg| arg.as_type())).is_some_and(|ty| ty.is_param(index))\n                                && body_may_drop(calls, (callee, at), seen)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_singles_across_crates"],
  },
  {
    name: "compiled-for-identities-unsaid",
    breaks:
      "a refusal of what a library function compiled as it merges what the crate compares can't reach says it's given a value to copy or drop, not why it's compiled (#1052)",
    file: "src/lower/instances/needed.rs",
    find: "        None if foreign.merges_identities(root) => \"one whose JS may give one function where Rust makes two, which this crate compares\".to_string(),\n",
    replace: "        None if false => \"one whose JS may give one function where Rust makes two, which this crate compares\".to_string(),\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "may merge what its consumer compares"],
  },
  {
    name: "closure-of-a-parameter-drops-nothing",
    breaks:
      "a library function that makes a closure of its type parameter's value and gives it back, `joined`, isn't compiled where it's given closures that drop, so its library's closure, carrying no drop, drops neither as `fold` ends (#1051)",
    file: "src/lower/instances/needed.rs",
    find: "    let drops = closes_over || drops;\n",
    replace: "    let drops = drops;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: closure_made_by_an_inlined_function"],
  },
  {
    name: "copy-reaches-private-state",
    breaks:
      "a copy of a library's function that reads its library's private state is taken to reach it: compiled here to keep a capacity, it's refused, where the library's own would run (#1180's (5))",
    file: "src/lower/instances/needed.rs",
    find: "        if !out_of_reach(tcx, foreign, body).is_empty() {\n            return false;\n        }\n",
    replace: "",
    tests: ["test/crates-compiled-here.test.ts", "-t", "keeps no capacity has its functions"],
  },
  {
    name: "copy-reaches-callees-unasked",
    breaks:
      "a copy of a library's function is asked only of its own body, not of the private function it calls, which reads its library's private state: compiled here, it's refused (#1180's (5))",
    file: "src/lower/instances/needed.rs",
    find: "        todo.extend(called(body).into_iter().map(|(callee, _)| callee));\n",
    replace: "",
    tests: ["test/crates-compiled-here.test.ts", "-t", "keeps no capacity has its functions"],
  },
];
