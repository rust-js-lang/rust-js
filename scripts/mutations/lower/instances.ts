// Mutations of src/lower/instances.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closure-placed-by-its-root-alone",
    breaks: "a closure an instance makes of a function rustc inlined into it is looked up by its typeck root alone, `joined`, which no module holds: a panic (#1029)",
    file: "src/lower/instances.rs",
    find: "            .or_else(|| self.placed.get(&id)))\n",
    replace: "            .or(None))\n",
    tests: ["test/crates.test.ts", "-t", "inlined function is placed"],
  },
  {
    name: "closure-of-an-inlined-function-unplaced",
    breaks: "`joined`'s closure, made in the instance `fold_with`, isn't placed where `fold_with` is: a panic of no module for it (#1029)",
    file: "src/lower/instances.rs",
    find: "            made.push((closure, module));\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "inlined function is placed"],
  },
  {
    name: "library-changed-unshared",
    breaks:
      "a library's exported function that copies what its consumer changes in place, which the library changes too, so its JS copies it, is compiled by the consumer, where it is imported, `Dep.firstNotch(notches)`; React's Flight client's `close`, refused as it reads its library's private state (rd606)",
    file: "src/lower/instances.rs",
    find: "                .filter(|&&ty| !self.recognition.foreign.changes(library, ty))\n",
    replace: "                .filter(|_| true)\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "a library's generic function is its program's",
    ],
  },
  {
    name: "instance-drop-ignored",
    breaks:
      "a consumer's call that gives a library's generic function a value with a destructor its JS takes no drop for calls the library's, so a panic in it drops nothing, where Rust drops `Loud(2)` (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "        (drops && !takes(index, ParamFact::Drop) && body_may_drop(calls, (callee, index), may_drop))\n",
    replace:
      "        (false && body_may_drop(calls, (callee, index), may_drop))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "instance-copy-ignored",
    breaks:
      "a consumer's call that gives a library's `T: Copy` function a `Point` it changes in place calls the library's, which copies nothing, so the two cells it made hold one object (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            || (copies && copied.contains(&index) && !takes(index, ParamFact::Copy))\n",
    replace: "            || (false && copied.contains(&index))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_given"],
  },
  {
    name: "instance-private-callee-imported",
    breaks:
      "a library's private function an instance calls isn't compiled with it, and is imported from a module that doesn't export it (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            let private = instance && foreign.item(callee).is_none();\n",
    replace: "            let private = false;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "instance-private-state-unrefused",
    breaks:
      "an instance that reads its library's private thread-local isn't refused, and reads one of its own, or none (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            refuse_private_state(&calls, body.def_id, &mir.body);\n",
    replace: "            let _ = refuse_private_state;\n",
    tests: ["test/crates.test.ts", "-t", "reads its library's private state"],
  },
  {
    name: "instance-copy-of-a-given-param-ignored",
    breaks:
      "a consumer's generic function given a copy of its `T` gives a library's function its `T` with none, as if its own copy (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            _ => !super::analysis::copies_itself(tcx, given),\n",
    replace: "            _ => false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_given"],
  },
  {
    name: "cleanup-always-unwound",
    breaks:
      "a library's function's drop where a binding that can't throw unwinds is taken for one, and a consumer compiles its own `count` that drops nothing (ADRs 0301, 0914)",
    file: "src/lower/instances.rs",
    find: "        _ if data.is_cleanup && !unwound.contains(&block) => false,\n",
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "can't throw, read before a move"],
  },
  {
    name: "copied-argument-not-given",
    breaks:
      "a value optimized MIR copies where it's last used, `pick::<(), T>(const (), copy _1)`, isn't taken for given, so `discard` drops nothing of a consumer's `Loud` (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "                    mir::Operand::Move(place) | mir::Operand::Copy(place) => lent(place.ty(&body.local_decls, tcx).ty),\n",
    replace: "                    mir::Operand::Move(place) => lent(place.ty(&body.local_decls, tcx).ty),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: drops_by_need"],
  },
  {
    name: "closure-call-may-drop",
    breaks:
      "a value a library's function gives a closure it's given is taken for one it may drop, and a consumer compiles `apply` and `keep_by`, which drop nothing (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "                    {\n                        false\n                    }\n                    None => false,\n",
    replace:
      "                    {\n                        true\n                    }\n                    None => true,\n",
    tests: ["test/crates.test.ts", "-t", "takes the drops its body uses"],
  },
  {
    name: "instance-constant-not-copied",
    breaks:
      "a library's function that reads its `const SHAPE`, a `Copy` enum its consumer changes in place, isn't taken to copy it, so changing what it gives changes the constant (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "                    mir::Operand::Constant(constant) => self.copies |= (self.holds)(constant.ty()),\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_enum"],
  },
  {
    name: "instance-box-read-copied",
    breaks:
      "optimized MIR's copy of a `Box` it reads through is taken for a copy of what it holds, and a consumer compiles the library's `grow` itself, `const Tree = { grow(tree) { .. } }`, which it needn't (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "                    mir::Operand::Copy(place) if copy(place.ty(&self.body.local_decls, tcx).ty) => {\n",
    replace:
      "                    mir::Operand::Copy(place) if { let _ = &copy; true } => {\n",
    tests: ["test/crates.test.ts", "-t", "grows a library.s boxed tree"],
  },
  {
    name: "instance-trait-call-unresolved",
    breaks:
      "a library function's call of its trait's method isn't resolved to the impl's that copies a `Spot`, so `first_of` is its library's and shares what it gives (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "                                .map_or(callee, |instance| instance.def_id()),\n",
    replace: "                                .map_or(callee, |_| callee),\n",
    tests: ["test/crates.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "instance-clones-unasked",
    breaks:
      "a consumer that gives a library's generic function a value that isn't its own clone, where its export takes no `TClone`, calls the export, whose clone is the value itself, and its copy changes what it was given (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "            || (clones && cloned.contains(&index) && !takes(index, ParamFact::Clone))\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "compiles its own of the function",
    ],
  },
  {
    name: "binding-result-copies",
    breaks:
      "a library's function whose binding call gives a `Vec<String>` the crate pushes to, `js::object::keys(props)`, is taken to copy it, and compiled again in the crate, React's PerformanceTrack's own `addObjectToProperties`",
    file: "src/lower/instances.rs",
    find: "                        Some(callee) if super::bindings::is_binding(tcx, callee) => false,\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "binding's Vec is imported",
    ],
  },
  {
    name: "concat-copies-its-array",
    breaks:
      "a library's function of a slice's `concat()` of `Option<&Unknown>`s, which the crate pushes a `Vec` of, is compiled again, React's addValueToProperties' 620 lines in the reconciler (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "ty::Adt(_, made) => made.types().next().is_some_and(|item| (self.holds)(item)),",
    replace: "ty::Adt(..) => true,",
    tests: ["test/cross-crate-items.test.ts", "-t", "of a concat is imported"],
  },
  {
    name: "concat-items-unasked",
    breaks:
      "a library's function of a `concat()` of `Vec<u32>`s the crate pushes to is imported, its library's JS sharing the arrays it was given, which the crate's push then changes (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "ty::Adt(_, made) => made.types().next().is_some_and(|item| (self.holds)(item)),",
    replace: "ty::Adt(_, made) => made.types().next().is_some_and(|_| false),",
    tests: ["test/cross-crate-items.test.ts", "-t", "of a concat is imported"],
  },
  {
    name: "instance-gives-nullish-unasked",
    breaks:
      "a consumer that gives a library's unboxed type parameter a `()` calls the export, whose `Some(())` is `undefined`, `None` (ADR 0051, amended)",
    file: "src/lower/instances.rs",
    find: "            || (nullish\n                && counted(index)\n",
    replace: "            || (false\n                && counted(index)\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "unboxed function"],
  },
  {
    name: "instance-binding-static-private",
    breaks:
      "a library's function its consumer compiles is refused where it reads a binding's static, a global JS has, as React DOM's call of the reconciler's runWithFiberInDEV reading shared's `__DEV__` was (ADR 0914, amended, gap rd75)",
    file: "src/lower/instances.rs",
    find: "if state && !binding && !def.is_local()",
    replace: "if state && !def.is_local()",
    tests: ["test/crates.test.ts", "-t", "private state"],
  },
  {
    name: "instance-relative-static-read",
    breaks:
      "a library's function its consumer compiles reads a binding's static its library binds to a module by a path from its own file, which the consumer's file doesn't name (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: ".chain(statics)",
    replace: ".chain(statics.take(0))",
    tests: ["test/crates.test.ts", "-t", "private state"],
  },
  {
    name: "instance-unit-given-copied",
    breaks:
      "a library's generic function given `()`s, which may look like `None`, is compiled by its consumer, where it makes no `Some` of them and its export serves, as React DOM copied the reconciler's runWithFiberInDEV (ADR 0914, amended, gap rd75b)",
    file: "src/lower/instances.rs",
    find: "                && body_may_box(calls, (callee, index), &mut IndexSet::default()))",
    replace: "                && true)",
    tests: ["test/crates.test.ts", "-t", "given what may look like None"],
  },
  {
    name: "instance-closure-holds-its-makers-params",
    breaks:
      "a closure a library's generic function gives std holds each of its type parameters, though it captures none, so React DOM copies the reconciler's runWithFiberInDEV, whose `CURRENT.with_borrow(|current| current.clone())` gives std no `()`, with a JS label and drop flags (ADR 0914, amended, gap rd85)",
    file: "src/lower/instances.rs",
    find: "                walk.skip_current_subtree();\n",
    replace: "                continue;\n",
    tests: ["test/crates.test.ts", "-t", "given what may look like None"],
  },
  {
    name: "instance-box-unseen",
    breaks:
      "a library's generic function that makes a `Some` of what it's given, `wrapped(())`, is called as its export, which boxes none, so its `Some(())` is `None` (ADR 0051, amended)",
    file: "src/lower/instances.rs",
    find: "    if body.local_decls.iter().any(|decl| boxes(decl.ty)) {\n",
    replace:
      "    if false && body.local_decls.iter().any(|decl| boxes(decl.ty)) {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: generic_given_unit"],
  },
  {
    name: "instance-sets-library-variable",
    breaks:
      "a library's function its consumer compiles sets its library's module variable, an import, which JS can't assign: a TypeError where Rust runs (ADR 0270, ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "if sets && let Some(&key)",
    replace: "if false && sets && let Some(&key)",
    tests: ["test/crates.test.ts", "-t", "private state"],
  },
  {
    name: "through-library-compiled",
    breaks:
      "a library's namespace's method called through a value is one called by its name, compiled into the consumer where its call needs a copy (ADR 0914), and refused there for its library's private state, not for the copy JS's value can't be (rd311)",
    file: "src/lower/instances.rs",
    find: "                through.then_some((callee, generic, *fn_span, destination.local))\n",
    replace: "                through.then_some((callee, generic, *fn_span, destination.local)).filter(|_| false)\n",
    tests: ["test/namespaces.test.ts", "-t", "through a value whose call needs a copy is refused"],
  },
  {
    name: "through-library-unrefused",
    breaks:
      "a library's namespace's method through a value whose call gives what may look like `None` and isn't flattened is called, its `Some(None)` read as `None` (rd311)",
    file: "src/lower/instances.rs",
    find: "                super::classes::refuse_through(tcx, callee, span, !other);\n",
    replace: "                let _ = (span, other);\n",
    tests: ["test/namespaces.test.ts", "-t", "refused unless its result is flattened"],
  },
  {
    name: "through-library-flattened-refused",
    breaks:
      "a library's namespace's method through a value whose result only `.flatten()` reads is refused (rd311)",
    file: "src/lower/instances.rs",
    find: "            if gives_more(&calls, given, &|index| Some(index) != flattened, &mut may_drop) {\n",
    replace: "            if gives_more(&calls, given, &|_| true, &mut may_drop) {\n",
    tests: ["test/namespaces.test.ts", "-t", "exact where its result is flattened"],
  },
  {
    name: "flattened-by-name-compiled-here",
    breaks:
      "a namespace's method by its name whose result is flattened is compiled in the consumer, which reads its library's private state, refused, react-test-renderer's `unstable_flushSync` (rd324)",
    file: "src/lower/instances.rs",
    find: "            let flattened = super::analysis::flattened_by_name(&calls.recognition, &mir.body, (callee, args));\n",
    replace: "            let flattened: Option<u32> = None;\n",
    tests: ["test/namespaces.test.ts", "-t", "by its name is its library"],
  },
  {
    name: "library-body-copies-asked-monomorphized",
    breaks:
      "what a library's generic function copies is asked with no parameters of its own, so a consumer of one whose `[i32; N]` it iterates crashes rustc, `cannot find N/#0 in param-env` (#454)",
    file: "src/lower/instances.rs",
    find: "            typing_env: ty::TypingEnv::post_analysis(tcx, body.source.def_id()),\n",
    replace: "            typing_env: ty::TypingEnv::fully_monomorphized(),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: const_generic"],
  },
  {
    name: "instance-impl-consts-unasked",
    breaks:
      "a library's trait impl a consumer compiles itself is compiled without its constants, so a library that compiles arrayvec's `impl<T, const CAP: usize> ArrayVecImpl for ArrayVec<T, CAP>`, rust_decimal, is refused at its `CAPACITY = CAP`, which has no initializer to lower (#468, ADR 0176, amended)",
    file: "src/lower/instances.rs",
    find: "                    DefKind::AssocConst { .. } => true,\n",
    replace: "                    DefKind::AssocConst { .. } => false,\n",
    tests: ["test/crates.test.ts", "-t", "three crates: generic_impl_const_of_instance"],
  },
  {
    name: "js-alone-given-counted",
    breaks:
      "`function::name(&signal_oper::<()>)` counts as a call giving `()`, so `signalOper` boxes its `Some` and `signal` binds a copy, `signalOperUnboxed`: `isSignal` is false (#565)",
    file: "src/lower/instances.rs",
    find: "        .filter(|&(callee, _, at)| !alone.iter().any(|&(f, given)| f == callee && given.contains(at)))\n",
    replace: "        .filter(|_| true)\n",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "js-alone-of-a-function-given-back",
    breaks:
      "a function `function::bind` gives back, which Rust calls, gives its type parameters nothing: `first::<()>`'s `Some(())` isn't boxed, so `is_some()` of it is false (#565)",
    file: "src/lower/instances.rs",
    find: "            && function_itself(tcx, callee, index) == Some(Itself::Unreturned)\n",
    replace: "            && function_itself(tcx, callee, index).is_some()\n",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "js-alone-of-no-rest",
    breaks:
      "a generic function of no rest arguments given to `function::name` gives its type parameters nothing, though its value is still lowered as its call's copy, which no analysis made: refused (#565)",
    file: "src/lower/instances.rs",
    find: "            && crate::lower::bindings::takes_rest(tcx, f)\n",
    replace: "",
    tests: ["test/rest-parameters.test.ts", "-t", "a generic function of rest arguments JS alone reads is its own JS"],
  },
  {
    name: "closure-constant-functions-uncalled",
    breaks: "a function a closure constant holds, std's `Iterator::last`'s `some` in `FlattenCompat`'s inlined `last` of `Option`s, is given no drop of its type parameter, so `flatten().last()` of `Some`s never drops the one it passes (#645)",
    file: "src/lower/instances.rs",
    find: "            } else if constant.ty().is_closure() {\n",
    replace: "            } else if false {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "copies-settled-alone",
    breaks:
      "of functions that call each other, only the last asked is settled with the first's answer: the first stays asked, and each asked of it after answers no copy, so `outer`, which calls `inner`, which calls it and one that copies, is the library's, sharing what Rust copies (#884)",
    file: "src/lower/instances.rs",
    find: "            let settled = self.asking.borrow_mut().drain(depth..).collect::<Vec<_>>();\n",
    replace: "            let settled = self.asking.borrow_mut().pop().into_iter().collect::<Vec<_>>();\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "a library's generic function is its program's",
    ],
  },
  {
    name: "impl-defaults-unasked",
    breaks:
      "whether a library's impl copies what the crate changes asks only the methods it writes, not its trait's defaults its exported dictionary holds, so a generic default's clone of a `Vec` the crate changes is shared (#1023)",
    file: "src/lower/instances.rs",
    find: "                .chain(defaults_used(tcx, id))\n",
    replace: "                .chain(defaults_used(tcx, id).filter(|_| false))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_own_methods_through_its_dictionary"],
  },
  {
    name: "derive-asked-as-its-library",
    breaks:
      "a library's derive a body calls, which rust-js writes in place of the caller's types, is asked generically as its library's code, so a `Holder<T>`'s clone where `T::Item` is a `Vec` the crate changes is shared (#1023)",
    file: "src/lower/instances.rs",
    find: "                        Some(callee) if self.recognition.foreign.is_library(callee.krate) && !derived(callee) => {\n",
    replace: "                        Some(callee) if self.recognition.foreign.is_library(callee.krate) => {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_own_methods_through_its_dictionary"],
  },
  {
    name: "writer-refusal-unnamed",
    breaks:
      "a writer compiled here for the options it's given, refused where it reads its library's private state, says it's compiled for a copy or a drop, not what it's given (#1026)",
    file: "src/lower/instances.rs",
    find: "    match foreign.writes_more(root) {\n",
    replace: "    match foreign.writes_more(root).filter(|_| false) {\n",
    tests: ["test/crates.test.ts", "-t", "refused where it reads its library's private state"],
  },
  {
    name: "items-given-drops-unasked",
    breaks:
      "a call giving a library's function an iterator of no destructor whose items have one, `second(Gen(0))`, gives it nothing more, so its export, which takes no drop for `I::Item`, never drops the item it passes over (#1031)",
    file: "src/lower/instances.rs",
    find: "    }) || items_given_drops(calls, (caller, callee, args), &takes)\n",
    replace: "    }) || (false && items_given_drops(calls, (caller, callee, args), &takes))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
  {
    name: "owned-param-given-no-drop",
    breaks:
      "a type that owns the caller's type parameter, a closure holding a `Vec<K>`, given a library's function, gives it nothing to drop where the caller is given a drop for `K`, so `boxed`'s `dyn Fn` drops nothing (#1031)",
    file: "src/lower/instances.rs",
    find: "            _ if given.has_param() => super::drops::owned_params(tcx, given).iter().any(caller_drops),\n",
    replace: "            _ if given.has_param() => false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
  {
    name: "drop-in-place-drops-nothing",
    breaks:
      "a library's body's `ptr::drop_in_place::<T>(p)` drops no `T` to `body_may_drop`, its `*mut T` owning none, so `Slots::truncate` is its export, which drops nothing (#1031)",
    file: "src/lower/instances.rs",
    find: "                    return callee_args.types().any(holds);\n",
    replace: "                    return false;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
  {
    name: "item-slot-callee-drops-nothing",
    breaks:
      "a library's body that gives its `M: Iterator<Item = T>` to a function of its library's that drops items, `second(made)`, drops no `T` to `body_may_drop`, so `made_second` is its export, which drops nothing (#1031)",
    file: "src/lower/instances.rs",
    find: "                    && compiled_here(calls.foreign, tcx, callee)\n                    && super::traits::drop_items(tcx, calls.foreign, callee)\n",
    replace: "                    && compiled_here(calls.foreign, tcx, callee)\n                    && false\n                    && super::traits::drop_items(tcx, calls.foreign, callee)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_drops_its_consumer_gives"],
  },
  {
    name: "lent-mut-drops-nothing",
    breaks:
      "a library's body that lends a `&mut Slot<T>` to what may write its `T` again, `slot.renew()`, drops no `T` to `body_may_drop`, so `renewed` is its export, which drops the old one nowhere (#1032)",
    file: "src/lower/instances.rs",
    find: "                    holds(ty) || matches!(*ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if holds(inner))\n",
    replace: "                    holds(ty)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_singles_across_crates"],
  },
  {
    name: "callee-given-holder-drops-nothing",
    breaks:
      "a library's body that gives what holds its `T`, not `T` itself, to a function of its library's drops no `T` to `body_may_drop` (#1032)",
    file: "src/lower/instances.rs",
    find: "                            (callee_args.get(at as usize).and_then(|arg| arg.as_type())).is_some_and(holds)\n                                && body_may_drop(calls, (callee, at), seen)\n",
    replace: "                            (callee_args.get(at as usize).and_then(|arg| arg.as_type())).is_some_and(|ty| ty.is_param(index))\n                                && body_may_drop(calls, (callee, at), seen)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_singles_across_crates"],
  },
  {
    name: "compiled-for-identities-unsaid",
    breaks:
      "a refusal of what a library function compiled as it merges what the crate compares can't reach says it's given a value to copy or drop, not why it's compiled (#1052)",
    file: "src/lower/instances.rs",
    find: "        None if foreign.merges_identities(root) => \"one whose JS may give one function where Rust makes two, which this crate compares\".to_string(),\n",
    replace: "        None if false => \"one whose JS may give one function where Rust makes two, which this crate compares\".to_string(),\n",
    tests: ["test/crates.test.ts", "-t", "may merge what its consumer compares"],
  },
  {
    name: "closure-of-a-parameter-drops-nothing",
    breaks:
      "a library function that makes a closure of its type parameter's value and gives it back, `joined`, isn't compiled where it's given closures that drop, so its library's closure, carrying no drop, drops neither as `fold` ends (#1051)",
    file: "src/lower/instances.rs",
    find: "    let drops = closes_over || drops;\n",
    replace: "    let drops = drops;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: closure_made_by_an_inlined_function"],
  },
  {
    name: "rc-param-impl-method-resolved",
    breaks: "a trait impl's generic method that counts an `Rc<T>`, called through its trait, isn't asked what the trait's method's namings give its own `T`, so it's taken for one given nothing, the `Rc` it's given isn't counted and its count is wrong (#1001, #1009)",
    file: "src/lower/instances.rs",
    find: "        if declared != owner {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(rc_counted_by_a_trait_method|rc_counted_by_callers_instances).rs$"],
  },
  {
    name: "rc-public-callers-seen",
    breaks: "a `pub` generic function's `Rc<T>` counts only what the crate's own calls give it, though JS or a consumer may give it any type, whose `Rc` is then uncounted there (#1009)",
    file: "src/lower/instances.rs",
    find: "        Fact::Shared => !tcx.visibility(owner).is_public(),\n",
    replace: "        Fact::Shared => true,\n",
    tests: ["test/crates.test.ts", "-t", "counts each type"],
  },
  {
    name: "promoted-fn-ref-unseen",
    breaks: "a library's generic function made a `&dyn Fn`, `&dep::by_dyn`, a promoted `&FnDef` in MIR, isn't seen as named, so the consumer's `dyn` is the library's unboxed export and `d(())` is `None` (#1210)",
    file: "src/lower/instances.rs",
    find: "            if let Some(called) = super::fn_def(constant.ty().peel_refs()) {\n",
    replace: "            if let Some(called) = super::fn_def(constant.ty()) {\n",
    tests: ["test/crates.test.ts", "-t", "library_some_through_a_dyn"],
  },
  {
    name: "relayed-copy-lost",
    breaks:
      "a type parameter passed on to a function that copies isn't given a copy itself, so `relay(p)` shares `p`'s object",
    file: "src/lower/instances.rs",
    find: "            if given.contains(&from) && given.insert(to) {",
    replace: "            if false && given.insert(to) {",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a Copy bound takes a copy only",
    ],
  },
  {
    name: "clone-given-of-its-own-clone",
    breaks:
      "a `T: Clone` given only what's its own clone, `Option<Rc<Cache>>`, still takes a `TClone`, and its clone under a library's unchecked borrow is refused (ADR 0914, amended)",
    file: "src/lower/instances.rs",
    find: "                    _ if itself(given_ty) => {}\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "judged by what its callers give",
    ],
  },
  {
    name: "trait-method-copies-by-callers",
    breaks:
      "a trait method's `Copy` bound is decided by the crate's direct calls, though it's called through a dictionary",
    file: "src/lower/instances.rs",
    find: "                && tcx.trait_of_assoc(id).is_none()\n                && tcx.trait_impl_of_assoc(id).is_none()\n",
    replace: "                && tcx.trait_impl_of_assoc(id).is_none()\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a Copy bound takes a copy only",
    ],
  },
  {
    name: "trait-impl-copies-by-callers",
    breaks:
      "a trait impl's own `T: Copy` takes no copy function, which its method's dictionary is given, so a copy of an `Option<Point>` its method makes is the value it copies, and a change to it shows there",
    file: "src/lower/instances.rs",
    find: "                && !matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^trait_impl_copy_bound.rs$"],
  },
  {
    name: "instance-copies-unasked",
    breaks:
      "a library's function the crate compiles itself takes a copy of each `Copy` type parameter, as one a dictionary calls, `pairOf(value, dropped, TCopy, dropD)` where its callers give its own copy (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            (id.is_local() || foreign.is_instance(id))\n",
    replace: "            id.is_local()\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "is its program's, alone or a dependency",
    ],
  },
];
