// Mutations of src/lower/body_queries/ranges/closures.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closure-told-what-it-changes",
    breaks:
      "a `move` closure that writes the `i` it took is told `i` as it was where it's made, so its `items[i]` past the end reads `undefined` where Rust panics (rd423)",
    file: "src/lower/body_queries/ranges/closures.rs",
    find: "        ty::UpvarCapture::ByValue => captured.mutability == ty::Mutability::Not,\n",
    replace: "        ty::UpvarCapture::ByValue => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^index_bounds_closures\\.rs$"],
  },
  {
    name: "closure-told-through-a-mutable-borrow",
    breaks:
      "a closure that writes the `i` it borrows `&mut` is told `i` as it was where it's made, so its `items[i]` past the end reads `undefined` where Rust panics (rd423)",
    file: "src/lower/body_queries/ranges/closures.rs",
    find: "        ty::UpvarCapture::ByRef(kind) => kind == ty::BorrowKind::Immutable,\n",
    replace: "        ty::UpvarCapture::ByRef(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^index_bounds_closures\\.rs$"],
  },
  {
    name: "closure-told-a-cell",
    breaks:
      "a closure is told a `RefCell`'s slice as long as where it's made, which code between may clear, so `cell.borrow()[i]` reads `undefined` where Rust panics (rd423)",
    file: "src/lower/body_queries/ranges/closures.rs",
    find: " && !path.contains(&CELL);\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^index_bounds_closures\\.rs$"],
  },
  {
    name: "closure-told-nothing",
    breaks:
      "a closure's walk knows nothing of what it captures where it's made, so `OBJECT_IS.with(|is| is(&next[i], &prev[i]))` checks both indexes (rd423)",
    file: "src/lower/body_queries/ranges/closures.rs",
    find: "            let told = told(self.tcx, closure, k);\n",
    replace: "            let (told, _) = (Known::default(), k);\n",
    tests: ["test/corpus.test.ts", "-t", "^index_bounds_closures\\.rs$"],
    snapshots: true,
  },
];
