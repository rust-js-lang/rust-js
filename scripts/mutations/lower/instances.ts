// Mutations of src/lower/instances.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
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
    find: "                mir::Operand::Move(place) | mir::Operand::Copy(place) => holds(place.ty(&body.local_decls, tcx).ty),\n",
    replace:
      "                mir::Operand::Move(place) => holds(place.ty(&body.local_decls, tcx).ty),\n",
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
    find: "            || (nullish\n                && takes(index, ParamFact::Unboxed)\n",
    replace:
      "            || (false\n                && takes(index, ParamFact::Unboxed)\n",
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
    find: "                && body_may_box(calls, (callee, index), &mut HashSet::new()))",
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
];
