// Mutations of crates/rust-js-mir/src/queries/reads.rs, where a body's MIR reads and writes a local or a place.
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "read-before-anything-runs-past-a-call",
    breaks:
      "a read past a call is taken for one before anything runs, so a counted guard's value is read after its `$unborrow` and the call that writes its cell, `chunk.id` after `touch` emptied it (#2579)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "        next.extend(data.terminator().successors().map(|b| (b, true)));\n",
    replace: "        next.extend(data.terminator().successors().map(|b| (b, ran)));\n",
    tests: ["test/tidy-stages.test.ts", "-t", "guard_value_read_before_write"],
    env: { TIDY_VARIANTS: "cells" },
  },
  {
    name: "read-before-anything-runs-past-what-does-nothing",
    breaks:
      "a statement that does nothing, a `StorageDead`, is taken to run code, so a counted guard's value is read before its release though nothing runs between, `const result = value.value; $unlock(value); return result;` (#2579)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "    let inert = |statement: &mir::Statement<'_>| {\n        matches!(\n",
    replace: "    let inert = |statement: &mir::Statement<'_>| {\n        false && matches!(\n",
    tests: ["test/corpus.test.ts", "-t", "^(locks|lock_poisoning).rs$"],
    snapshots: true,
  },
  {
    name: "read-before-anything-runs-blind-to-a-terminator's-reads",
    breaks:
      "a terminator's read past what ran is taken for one before it, so a counted guard's value given a call past another is read after its `$unborrow` (#2579)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "        match first_use_by_terminator(body, block, local) {\n            Some(true) if ran => return false,\n",
    replace: "        match first_use_by_terminator(body, block, local) {\n            Some(true) if false => return false,\n",
    tests: ["test/tidy-stages.test.ts", "-t", "guard_value_read_before_write"],
    env: { TIDY_VARIANTS: "cells" },
  },
  {
    name: "read-before-anything-runs-blind-to-reads",
    breaks:
      "a statement's read past what ran is taken for one before it, so a counted guard's value is read after its `$unborrow` and the call that writes its cell (#2579)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "            match first_use(statement, at, local) {\n                Some(true) if ran => return false,\n",
    replace: "            match first_use(statement, at, local) {\n                Some(true) if false => return false,\n",
    tests: ["test/tidy-stages.test.ts", "-t", "guard_value_read_before_write"],
    env: { TIDY_VARIANTS: "cells" },
  },
  {
    name: "item-reach-of-another-local",
    breaks: "a write of another local's item, `other[0] = ..`, is taken to change what `cur[0]` reaches, so a place is held where it's made though nothing sets it (#1075)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "    (written.local == place.local && crate::rvalues::within(place.projection, written.projection))\n",
    replace: "    crate::rvalues::within(place.projection, written.projection)\n",
    tests: ["test/corpus.test.ts", "-t", "^(reference_a_field_holds_set_again|reference_an_item_holds_set_again|given_last_unwinds|cache_node_walk).rs$"],
    snapshots: true,
  },
  {
    name: "mir-capture-copied-never",
    breaks:
      "a closure made in a loop that changes what it captures reads the variable, every closure the last value",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "        next.extend(data.terminator().successors());\n",
    replace: "",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "captures a variable itself where nothing",
    ],
  },
  {
    name: "read-before-written-past-a-write",
    breaks:
      "`node.v.set({ node = b; .. })` sets `b`'s, not the node `node` was where the `Cell` was reached",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "            if written_or_lent(statement).any(&written) {\n                return false;\n",
    replace: "            if written_or_lent(statement).any(&written) {\n                continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^given_last_unwinds.rs$"],
  },
  {
    name: "read-from-past-assignment",
    breaks:
      "a read of a local past where it's assigned again, a loop's `&mut found` made anew before it's read, is taken for one of the value written, so `found.push(v)` holds it first, `const f = found; f.push(v)` (#198)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "                Some(true) => return true,\n                Some(false) => continue 'blocks,\n",
    replace: "                Some(true) => return true,\n                Some(false) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^if_let_mut_else_assign.rs$"],
    snapshots: true,
  },
  {
    name: "written-before-read-blind",
    breaks:
      "a binding of a `mut` parameter written before it's read, `shape = ..; n`, is read in place, its parameter held whole before it's written, `const shape$1 = shape;`, not as the binding it is (#121)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "    uses.writes.iter().any(|write| {\n",
    replace: "    false && uses.writes.iter().any(|write| {\n",
    tests: ["test/corpus.test.ts", "-t", "^settled_until_written.rs$"],
    snapshots: true,
  },
  {
    name: "written-before-read-anywhere-after",
    breaks:
      "a parameter written after every read of a binding of it, after its `match`, keeps a `const n = shape._0;` in each arm (#121)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "(after || reached.contains(&write.block)) && reaching.contains(&write.block)",
    replace: "(after || reached.contains(&write.block))",
    tests: ["test/corpus.test.ts", "-t", "^settled_until_written.rs$"],
    snapshots: true,
  },
  {
    name: "write-through-reference-unseen",
    breaks:
      "a write through a `&mut`, `(*inner).s = ..`, isn't a write of `inner`, so a reference read of its field before it is read after it",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "                self.writes = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^references_read_before_their_field_is_written.rs$"],
  },
  {
    name: "held-reference-of-field-unseen",
    breaks:
      "a reference read through one a field holds, `&*inner.s`, is taken for one of a place borrowck keeps the same, so it's read after the field's written",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "            && !base.projection.is_empty()\n            && matches!(base.ty(body, tcx).ty.kind(), ty::Ref(_, _, ty::Mutability::Not))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^references_read_before_their_field_is_written.rs$"],
  },
  {
    name: "written-or-lent-lends-nothing",
    breaks:
      "`node.v.set({ let mut swap = || node = &b; swap(); 30 })` sets `b`'s, a `&mut node` lent to be written taken for no write of it, and a field's reference a closure sets, `|| node.0 = b`, never set again (#1034)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "        Rvalue::Ref(_, BorrowKind::Mut { .. }, lent) | Rvalue::RawPtr(mir::RawPtrKind::Mut, lent) => Some(lent),\n",
    replace:
      "        Rvalue::Ref(_, BorrowKind::Mut { .. }, lent) | Rvalue::RawPtr(mir::RawPtrKind::Mut, lent) if false => Some(lent),\n",
    tests: ["test/corpus.test.ts", "-t", "^reference_a_field_holds_set_again.rs$"],
  },
  {
    name: "read-before-written-box-content-unwritten",
    breaks: "a `Cell` reached through a box of a reference, set after `*node = b`, or through a field's reference set after `node.0 = b`, is taken for read before what it's reached through is written, so it sets the new node's (#980, #1034)",
    file: "crates/rust-js-mir/src/queries/reads.rs",
    find: "    (written.local == place.local && crate::rvalues::within(place.projection, written.projection))\n",
    replace: "    (written.local == place.local && written.projection.is_empty())\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(box_read_before_its_write|reference_a_field_holds_set_again).rs$",
    ],
  },
];
