// Mutations of crates/rust-js-lower/src/representation/cells.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cell-field-unhandled",
    breaks:
      "a `Cell` field is read as the cell, so `c.count.get()` reads `c.count.value` of the number it holds",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "            Some(_) => Expr::handle(read),\n",
    replace: "            Some(_) => read,\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "cell in a field is the property",
    ],
  },
  {
    name: "cell-field-boxed",
    breaks:
      "a `Cell` made into a field is `{ value: 1 }`, the box, where the field is what it holds",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "            Some(_) => Expr::held(value),\n",
    replace: "            Some(_) => value,\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "cell in a field is the property",
    ],
  },
  {
    name: "once-field-boxed",
    breaks:
      "a `OnceCell` in a field is a `{ value }` of its own, `_ioInfo: {}` where it's made and `payload._ioInfo.value` where it's read",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "        (self.is_std_type(ty, StdItem::OnceCell)).then(|| Ty::new_option(self.tcx, args.type_at(0)))\n",
    replace:
      "        (false && self.is_std_type(ty, StdItem::OnceCell)).then(|| Ty::new_option(self.tcx, args.type_at(0)))\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_fields.rs$"],
    snapshots: true,
  },
  {
    name: "kept-cell-never",
    breaks:
      "a struct's `RefCell` field lent is a handle made where it's read, a cell of its own each time, which counts none of the field's other borrows, so a borrow held through it meets none (ADR 0863, amended)",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "            || self.quiet_field(field.did)\n",
    replace: "            || true\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(refcell_field_lent_held|refcell_bound_field_held).rs$",
    ],
  },
  {
    name: "kept-cell-of-quiet-fields",
    breaks:
      'a struct\'s `RefCell` field no borrow of which can meet another is its kept cell anyway, `$fieldCell(fiber, "return", true)`, where its borrows are the property (ADR 0863, amended)',
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "            || self.quiet_field(field.did)\n",
    replace: "            || false\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "kept-cell-nullable-undefined",
    breaks:
      "a nullable field's kept cell writes its `None` `undefined`, where JS's fiber's `return` is `null` (ADR 0863, amended)",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "            given.push(Expr::bool(true));\n",
    replace: "",
    tests: ["test/lowering-field-cells.test.ts", "-t", "lent is its one cell"],
  },
  {
    name: "uncounted-nullable-stored-as-given",
    breaks:
      "a nullable field's kept cell borrowed where nothing's counted is a handle that writes its `None` `undefined` (ADR 0863, amended)",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "                true => super::NoneForm::Null.storage(),\n",
    replace: "                true => super::NoneForm::Undefined.storage(),\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a nullable RefCell field's None is null where it's written",
    ],
  },
  {
    name: "cell-field-getter-read-defined",
    breaks: "a nullable cell field of a struct only JS makes, which a getter may answer `undefined`, is read as never `undefined`, so what it held is stored in another nullable place as it is, `to.row = row`, where `row ?? null` (#1933)",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: " && !self.fields_answered_by_getters(ty)",
    replace: "",
    tests: ["test/lowering-nullable-tests.test.ts", "-t", "old value and get"],
  },
  {
    name: "placed-field-read-as-its-place",
    breaks:
      "a field of the crate's iterator held in a place of its own, `a: I` of `Both<Tri, J>`, is read as the place, not what's in it: `match k.a` tests `{ value: \"B\" }` against `\"B\"` (#2255)",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "        if field.is_some_and(|field| self.placed_field(field, ty)) {\n            return Expr::held(read);\n",
    replace: "        if false {\n            return Expr::held(read);\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_field_in_its_place_copied_cloned_compared\\.rs$"],
  },
  {
    name: "placed-field-made-bare",
    breaks:
      "a field of the crate's iterator held in a place of its own is made as the value, not a place: a derived `Default` of `Both<Tri, Tri>` is `{ a: \"A\" }`, whose own steps throw (#2255)",
    file: "crates/rust-js-lower/src/representation/cells.rs",
    find: "        if field.is_some_and(|field| self.placed_field(field, ty)) {\n            return Expr::object(",
    replace: "        if false {\n            return Expr::object(",
    tests: ["test/corpus.test.ts", "-t", "^iterator_field_in_its_place_copied_cloned_compared\\.rs$"],
  },
];
