// Mutations of crates/rust-js-lower/src/recognition/compiled_std/walk.rs (ADR 0093): finding
// std's bodies `@rust-js/std` compiles by walking std's MIR, split from compiled_std.rs (#1334).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "private-callee-not-compiled",
    breaks:
      "a function std keeps private that a compiled body calls, `Fuse`'s `and_then_or_clear`, isn't compiled with it, and the call is refused",
    file: "crates/rust-js-lower/src/recognition/compiled_std/walk.rs",
    find: "                    && ((!tcx.visibility(called).is_public()\n",
    replace:
      "                    && ((!tcx.visibility(called).is_public()\n                        && false\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "named-private-fns-uncompiled",
    breaks: "a function std keeps private that a compiled body names as a value, `FlattenCompat::last`'s own `last`, isn't compiled with it, so `Flatten::last` isn't (#645)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/walk.rs",
    find: "        for value in named_fns(body, &blocks) {\n",
    replace: "        for value in Vec::<DefId>::new() {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "question-mark-try-compiled",
    breaks: "a `?` of a `Result` in a compiled body, `Chain::read`'s, has std's `Try::branch` compiled for it, which the lowering writes itself, and `@rust-js/std` can't be built (#1156)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/walk.rs",
    find: "            if super::is_question_mark_written(tcx, callee, args) {\n",
    replace: "            if false && super::is_question_mark_written(tcx, callee, args) {\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "private-debug-shown-uncompiled",
    breaks: "a `Flatten`'s compiled `Debug` writes its private `FlattenCompat`, whose `Debug` the catalogue's walk leaves out, as the `&dyn Debug` it's given is no call: `@rust-js/std`'s build refuses it (#2252)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/walk.rs",
    find: "        found.extend(shown_private(tcx, typing_env, body, &blocks));\n",
    replace: "        let _ = shown_private(tcx, typing_env, body, &blocks);\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies as the compiler compiles them"],
  },
];
