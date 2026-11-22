// Mutations of crates/rust-js-lower/src/analysis/intervals/fields.rs, a struct's fields a
// reference parameter is read through, or a `RefCell` guard lends, as
// locals (ADR 1137, #1672 M3, step 5, #1815).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/analysis/intervals/fields.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "a struct's fields read through"];
const guardTests = ["test/corpus.test.ts", "-t", "refcell_guard_fields"];

export const mutations: Mutation[] = [
  {
    name: "fields-not-locals",
    breaks: "a struct's fields aren't locals, so `self.0.remove(0)` past `!self.0.is_empty()` keeps its check, a call-bounds line, and `self.items[self.at]` isn't proved",
    file,
    find: "    if pointers.is_empty() {\n",
    replace: "    if true || pointers.is_empty() {\n",
    tests,
  },
  {
    name: "fields-whole-uses-ignored",
    breaks: "a parameter given on whole, `self.reset()`, keeps its fields locals, so `self.items[self.at]` past it is taken in bounds though `reset()` cleared them, a proved-call-bounds line",
    file,
    find: "        !uses.written.contains(&pointer) && (!uses.whole.contains(&pointer) || frozen(pointer))\n",
    replace: "        !uses.written.contains(&pointer)\n",
    tests,
  },
  {
    name: "fields-kept-beside-a-mut-reference-used-whole",
    breaks: "a `&mut` parameter used whole keeps its fields as locals, so `node.children[0]` past `reset(node)` is taken in bounds, a proved-call-bounds line, and would read past an emptied field",
    file: "crates/rust-js-lower/src/analysis/intervals/fields.rs",
    find: "    let frozen = |pointer: Local| matches!(*body.local_decls[pointer].ty.kind(), ty::Ref(_, to, ty::Mutability::Not) if to.is_freeze(tcx, typing_env));\n",
    replace: "    let frozen = |pointer: Local| matches!(*body.local_decls[pointer].ty.kind(), ty::Ref(..));\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a shared reference's fields"],
  },
  {
    name: "fields-of-a-shared-reference-used-whole-forgotten",
    breaks: "a shared reference parameter used whole keeps none of its fields as locals, so `node.children[0]` under `node.children.len() == 1` beside `node` returned keeps its check, a call-bounds line",
    file: "crates/rust-js-lower/src/analysis/intervals/fields.rs",
    find: "        !uses.written.contains(&pointer) && (!uses.whole.contains(&pointer) || frozen(pointer))\n",
    replace: "        !uses.written.contains(&pointer) && (!uses.whole.contains(&pointer) || (false && frozen(pointer)))\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a shared reference's fields"],
  },
  {
    name: "fields-of-a-written-parameter",
    breaks: "a shared reference parameter the body makes another's, `h = other`, keeps its fields as locals, so `h.items[k]` under `k < h.items.len()` past it is unchecked and reads `undefined` where Rust panics (#1878)",
    file: "crates/rust-js-lower/src/analysis/intervals/fields.rs",
    find: "            if place.projection.is_empty() && context.is_mutating_use() {\n",
    replace: "            if false && place.projection.is_empty() && context.is_mutating_use() {\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "slice_field, tested below"],
  },
  {
    name: "guard-fields-not-locals",
    breaks: "a `RefCell` guard's fields aren't locals, so the undo stack's `history.index -= 1` under `history.index != 0` keeps its wrap, `(history.index - 1) >>> 0`, not `history.index--` (#1815)",
    file,
    find: "    if guards.is_empty() {\n",
    replace: "    if true || guards.is_empty() {\n",
    tests: guardTests,
    snapshots: true,
  },
  {
    name: "guard-fields-of-a-guard-made-twice",
    breaks: "a guard's local made another cell's guard, `history = second.borrow_mut()`, keeps what the first borrow knew of its fields, so `history.index -= 1` past it gives -1 where Rust wraps to 4294967295 (#1815)",
    file,
    find: "    guards.retain(|guard| once(guard));\n",
    replace: "    guards.retain(|_| true);\n",
    tests: guardTests,
  },
  {
    name: "guard-fields-of-a-guard-lent-whole",
    breaks: "a guard lent whole to a function, `step(&mut history)`, which sets its index, keeps its fields locals, so `history.index -= 1` past it gives -1 where Rust wraps to 4294967295 (#1815)",
    file,
    find: "                if at != location && !derefs && !shared {\n",
    replace: "                if false && at != location && !derefs && !shared {\n",
    tests: guardTests,
  },
  {
    name: "guard-fields-of-a-guard-lent-mut-as-shared",
    breaks: "a `&mut` of a guard lent whole is taken for a shared one, so `step(&mut history)`, which sets its index, keeps its fields locals, and `history.index -= 1` past it gives -1 where Rust wraps (#1944's guard, #1815)",
    file,
    find: "                let shared = matches!(kind, mir::BorrowKind::Shared);\n",
    replace: "                let shared = true;\n",
    tests: guardTests,
  },
  {
    name: "guard-fields-of-a-guard-lent-shared-whole",
    breaks: "a shared `&guard` a closure captures, `history.open` read, makes the guard's fields no locals, so the undo stack's `remove(0)` past a push keeps its check, `$removeOutOfBounds`, where upstream writes `history.entries.shift()` (#1944)",
    file,
    find: "                if at != location && !derefs && !shared {\n",
    replace: "                if at != location && !derefs {\n",
    tests: ["test/corpus.test.ts", "-t", "^guard_lent_to_a_closure_proves_remove\\.rs$"],
    snapshots: true,
  },
  {
    name: "guard-fields-of-a-pointer-lent-whole",
    breaks: "a guard's `DerefMut` given whole to a method, `history.rewind()`, which sets its index, keeps its fields locals, so `history.index -= 1` past it gives -1 where Rust wraps to 4294967295 (#1815)",
    file,
    find: "                    _ => {\n                        self.whole.insert(guard);\n                    }\n",
    replace: "                    _ => {}\n",
    tests: guardTests,
  },

];
