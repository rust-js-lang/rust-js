// Mutations of crates/rust-js-lower/src/analysis/cell_borrows/may_be_cell.rs (ADR 0093):
// whether two cells' types may be one cell. Split from cell_borrows.ts with
// its source.
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cell-may-be-ignores-dyn",
    breaks:
      "a `RefCell<dyn Shape>` held isn't the `RefCell<Circle>` it was made of, so a borrow of the concrete one runs on where Rust panics",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/may_be_cell.rs",
    find: "        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) if unsized_here => {\n            let (dyn_ty, other) = if matches!(a.kind(), ty::Dynamic(..)) {\n                (a, b)\n            } else {\n                (b, a)\n            };\n            matches!(other.kind(), ty::Dynamic(..)) || may_be_made(tcx, other, dyn_ty)\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_dyn_alias_borrow.rs$"],
  },
  {
    name: "cell-dyn-anywhere",
    breaks:
      "a `dyn` in a `Vec`'s items is taken to be any type, so a `RefCell<Vec<Rc<Node>>>` may be React's commit hooks' `RefCell<Vec<Rc<dyn Fn()>>>`, borrowed while they run, and each borrow of it is checked (#374, ADR 0960)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/may_be_cell.rs",
    find: "        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) if unsized_here => {\n",
    replace: "        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) if !unsized_here => true,\n        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) => {\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "field.s hooks held while they run meet no cell",
    ],
  },
  {
    name: "cell-unsized-tail-unseen",
    breaks:
      "a `RefCell<T>`'s `T` isn't where a value is made unsized in place, so a `RefCell<dyn Shape>` isn't the `RefCell<Circle>` it was made of, and a borrow of the concrete one runs on where Rust panics (ADR 0960)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/may_be_cell.rs",
    find: "            ty::Param(param) => return Some(param.index as usize),\n",
    replace: "            ty::Param(_) => return None,\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_dyn_alias_borrow.rs$"],
  },
  {
    name: "dyn-cell-any-trait",
    breaks: "a `dyn Named` in a cell may be a concrete type that implements no `Named`, `Framed<Circle>`, so a `RefCell<Framed<dyn Shape>>` beside it is boxed and refused (#1006)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/may_be_cell.rs",
    find: "    infcx.predicate_may_hold(&obligation)\n}\n",
    replace: "    let _ = infcx.predicate_may_hold(&obligation);\n    true\n}\n",
    tests: ["test/corpus.test.ts", "-t", "^ref_cell_of_a_dyn_tail_beside_a_ref_cell_of_a_dyn.rs$"],
  },
];
