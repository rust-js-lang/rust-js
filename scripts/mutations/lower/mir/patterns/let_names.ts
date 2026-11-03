// Mutations of src/lower/mir/patterns/let_names.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "own-field-binding-of-any-name",
    breaks:
      "a let chain's binding named otherwise than the field it reads, `prev_state` of `current.memoized_state`, is taken for one of the field's own name and read again in its place, its `const` gone (rd226, rd706)",
    file: "src/lower/mir/patterns/let_names.rs",
    find: "                && read_field(*expr) == Some(binding)\n",
    replace: "                && read_field(*expr).is_some()\n",
    tests: ["test/lowering-moves.test.ts", "-t", "binding named otherwise than its field keeps its const"],
  },
  {
    name: "own-field-binding-not-through-calls",
    breaks:
      "a let chain's binding of a `Cell` field's `get()`, `debug_task` of `fiber.debug_task.get()`, isn't taken for one of the field's own name, so it stays `const debugTask` where upstream reads `fiber._debugTask.run(..)` (ADR 0866, amended, rd706)",
    file: "src/lower/mir/patterns/let_names.rs",
    find: "                    thir::ExprKind::Call { args, .. } if !args.is_empty() => through(args[0]),\n",
    replace: "",
    tests: ["test/lowering-written-branches.test.ts", "-t", "binding of a field the test read is read again"],
  },
  {
    name: "frozen-borrow-of-a-cell",
    breaks:
      "a `let` of a `&Cell` is taken for a frozen borrow and read where it's made, `const count = store.count`, though a call it's held across sets it: the value before the call (rd658, P0)",
    file: "src/lower/mir/patterns/let_names.rs",
    find: "            held.is_some_and(|held| !held.has_escaping_bound_vars() && held.is_freeze(self.tcx, self.typing_env))\n",
    replace: "            held.is_some_and(|held| !held.has_escaping_bound_vars())\n",
    tests: ["test/corpus.test.ts", "-t", "^frozen_borrow_named.rs$"],
    snapshots: true,
  },
  {
    name: "frozen-borrow-not-of-a-ref",
    breaks: "a `RefCell`'s `Ref` isn't taken for a frozen borrow, so one held across a call loses its name (rd658)",
    file: "src/lower/mir/patterns/let_names.rs",
    find: "                ty::Adt(_, args) if self.is_std_type(ty, StdItem::RefCellRef) => args.types().next(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^frozen_borrow_named.rs$"],
    snapshots: true,
  },
  {
    name: "ended-borrow-of-any-let",
    breaks:
      "a `let` of a `Cell`'s value, `let index = current.index.get();`, is taken for one that ends a borrow, its name lost to its place's read",
    file: "src/lower/mir/patterns/let_names.rs",
    find: "                if initializer.and_then(borrow_cloned) == Some(binding) {\n",
    replace: "                if initializer.is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_read_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "guard-lets-unseen",
    breaks:
      "no `let` is a drop guard's, so its `try` starts at its first statement, a comment after the `let` before it (ADR 0865, amended)",
    file: "src/lower/mir/patterns/let_names.rs",
    find: "                    && adt.has_dtor(self.tcx)\n",
    replace: "                    && false\n",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a guard's try starts at its let",
    ],
  },
  {
    name: "ended-borrow-of-another-name",
    breaks:
      "a `let` that ends a borrow named otherwise than the field it reads, React's `const pendingFirst = pendingQueue.next; baseQueue.next = pendingFirst;`, loses the program's name to the place's read (ADR 0866, amended)",
    file: "src/lower/mir/patterns/let_names.rs",
    find: "                if initializer.and_then(borrow_cloned) == Some(binding) {\n",
    replace:
      "                if initializer.and_then(borrow_cloned).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_read_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "copied-field-binding-own-let",
    breaks: "`let x = *x;` right after a pattern is a `let` of its own, which splits the pattern's destructuring in three, `const { handler } = r; const parentObject = r.parentObject; const { key } = r;` (#546)",
    file: "src/lower/mir/patterns/let_names.rs",
    find: "                if !copied_field(binding, *initializer) {\n",
    replace: "                if !copied_field(binding, *initializer) || true {\n",
    tests: ["test/corpus.test.ts", "-t", "^pattern_binding_copied.rs$"],
    snapshots: true,
  },
];
