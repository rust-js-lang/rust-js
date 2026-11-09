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
  {
    name: "library-field-accessor-unread",
    breaks:
      "a library's class's trait method that only gives its field, which has no JS of its own (rd304), is called by its consumer, `holder.slot()`, a method its prototype hasn't (#1073)",
    file: "src/lower/classes/accessors.rs",
    find: "        return field_accessor_of_mir(tcx, def_id);\n",
    replace: "        return None;\n",
    tests: ["test/crates.test.ts", "-t", "library_trait_of_classes_given_fields|quiet field"],
  },
  {
    name: "library-field-accessor-past-its-end",
    breaks:
      "a library's method that drops a value after it borrows its field, `let _noisy = Noisy; &self.n`, is read as its field, and the drop never prints (#1073)",
    file: "src/lower/classes/accessors.rs",
    find: "    if !matches!(block.terminator().kind, mir::TerminatorKind::Return) {\n        return None;\n    }\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "library_trait_of_classes_given_fields"],
  },
  {
    name: "library-field-accessor-after-statements",
    breaks:
      "a library's method that sets a field before it gives one, `self.seen = true; &self.seen`, is read as its field, and the field is never set (#1073, #1074)",
    file: "src/lower/classes/accessors.rs",
    find: "            _ => return None,\n        }\n    }\n    let given = given.filter(",
    replace: "            _ => {}\n        }\n    }\n    let given = given.filter(",
    tests: ["test/crates.test.ts", "-t", "library_trait_of_classes_given_fields"],
  },
  {
    name: "library-field-accessor-of-a-mut",
    breaks:
      "a library's method that gives a `&mut` of its field, `&mut self.n`, is read as the field borrowed shared (#1073)",
    file: "src/lower/classes/accessors.rs",
    find: "                let mir::Rvalue::Ref(_, mir::BorrowKind::Shared, place) = &assign.1 else {\n",
    replace: "                let mir::Rvalue::Ref(_, _, place) = &assign.1 else {\n",
    tests: ["test/crates.test.ts", "-t", "library_trait_of_classes_given_fields"],
  },
  {
    name: "library-field-accessor-of-another-parameter",
    breaks:
      "a library's method that gives a field of another parameter, `&other.n`, is taken for one that gives its own, so holding it as a function is refused (#1073)",
    file: "src/lower/classes/accessors.rs",
    find: "    let given = given.filter(|place| place.local.as_usize() == 1)?;\n",
    replace: "    let given = given?;\n",
    tests: ["test/crates.test.ts", "-t", "library_trait_of_classes_given_fields"],
  },
];
