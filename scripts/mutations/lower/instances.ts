// Mutations of src/lower/instances.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "instance-drop-ignored",
    breaks: "a consumer's call that gives a library's generic function a value with a destructor its JS takes no drop for calls the library's, so a panic in it drops nothing, where Rust drops `Loud(2)` (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "        (drops && !takes(index, ParamFact::Drop) && body_may_drop(calls, (callee, index), may_drop))\n",
    replace: "        (false && body_may_drop(calls, (callee, index), may_drop))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "instance-copy-ignored",
    breaks: "a consumer's call that gives a library's `T: Copy` function a `Point` it changes in place calls the library's, which copies nothing, so the two cells it made hold one object (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            || (copies && copied.contains(&index) && !takes(index, ParamFact::Copy))\n",
    replace: "            || (false && copied.contains(&index))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_given"],
  },
  {
    name: "instance-private-callee-imported",
    breaks: "a library's private function an instance calls isn't compiled with it, and is imported from a module that doesn't export it (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            let private = instance && foreign.item(callee).is_none();\n",
    replace: "            let private = false;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "instance-private-state-unrefused",
    breaks: "an instance that reads its library's private thread-local isn't refused, and reads one of its own, or none (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            refuse_private_state(&calls, body.def_id, &mir.body);\n",
    replace: "            let _ = refuse_private_state;\n",
    tests: ["test/crates.test.ts", "-t", "reads its library's private state"],
  },
  {
    name: "instance-copy-of-a-given-param-ignored",
    breaks: "a consumer's generic function given a copy of its `T` gives a library's function its `T` with none, as if its own copy (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "            _ => !super::analysis::copies_itself(tcx, given),\n",
    replace: "            _ => false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_given"],
  },
  {
    name: "cleanup-always-unwound",
    breaks: "a library's function's drop where a binding that can't throw unwinds is taken for one, and a consumer compiles its own `count` that drops nothing (ADRs 0301, 0914)",
    file: "src/lower/instances.rs",
    find: "        _ if data.is_cleanup && !unwound.contains(&block) => false,\n",
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "can't throw, read before a move"],
  },
  {
    name: "copied-argument-not-given",
    breaks: "a value optimized MIR copies where it's last used, `pick::<(), T>(const (), copy _1)`, isn't taken for given, so `discard` drops nothing of a consumer's `Loud` (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "                mir::Operand::Move(place) | mir::Operand::Copy(place) => holds(place.ty(&body.local_decls, tcx).ty),\n",
    replace: "                mir::Operand::Move(place) => holds(place.ty(&body.local_decls, tcx).ty),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: drops_by_need"],
  },
  {
    name: "closure-call-may-drop",
    breaks: "a value a library's function gives a closure it's given is taken for one it may drop, and a consumer compiles `apply` and `keep_by`, which drop nothing (ADR 0914)",
    file: "src/lower/instances.rs",
    find: "                    {\n                        false\n                    }\n                    None => false,\n",
    replace: "                    {\n                        true\n                    }\n                    None => true,\n",
    tests: ["test/crates.test.ts", "-t", "takes the drops its body uses"],
  },
];
