// Mutations of src/lower/classes/accessors.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-accessor-reborrow-refused",
    breaks:
      "a method that gives a field, `&self.internal_root` reborrowed where it's given back, is no field accessor: a prototype method, `$fieldCell(this, ..)` (rd304)",
    file: "src/lower/classes/accessors.rs",
    find: "                && matches!(borrow.kind, Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not))) => {}\n",
    replace: "                && false => {}\n",
    tests: ["test/classes-traits.test.ts", "-t", "prototype's"],
  },
  {
    name: "field-accessor-of-any-path",
    breaks:
      "a method that gives what isn't a field of its `self`, `&OTHER.f`, or of a field reached through another reference, is read as a field of `self` (rd304)",
    file: "src/lower/classes/accessors.rs",
    find: "                    [deref] if is_self && matches!(deref.kind, Adjust::Deref(DerefAdjustKind::Builtin)) => true,\n",
    replace: "                    [_] => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_field_borrows.rs$"],
  },
  {
    name: "class-trait-unknown-self-any",
    breaks:
      "a default's call of a trait of classes' method, of a `Self` not known, may borrow anything, so a borrow held across `self.internal_root()` is counted and checked, `$borrow($fieldCell(this, ..))` (rd304)",
    file: "src/lower/classes/accessors.rs",
    find: "        Ok(None) => (implementations(tcx, callee).into_iter().flatten())\n",
    replace: "        Ok(None) => (implementations(tcx, callee).into_iter().flatten().filter(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_field_borrows.rs$"],
  },
  {
    name: "field-accessor-after-statements",
    breaks:
      "a trait of classes' method that runs statements before it gives a field, `self.seen = true; &self.seen`, is read as that field, `gate.seen`, and its statements never run (#1074)",
    file: "src/lower/classes/accessors.rs",
    find: "        if !block.stmts.is_empty() {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_method_before_its_field.rs$"],
  },
];
