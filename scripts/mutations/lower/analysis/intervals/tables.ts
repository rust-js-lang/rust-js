// Mutations of crates/rust-js-lower/src/analysis/intervals/tables.rs, a table's lengths
// and its rows' (ADR 1137, #1672 M3, step 5; ADR 0576).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/analysis/intervals/tables.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "a table is as long"];

export const mutations: Mutation[] = [
  {
    name: "rows-kept-of-any-row-lent",
    breaks: "a row's `&mut` used any way keeps the rows, so `board[0][c]` past `board[0].truncate(0)` or `mem::take(&mut board[0])` is taken in bounds, a proved-call-bounds line, and would read past an emptied row",
    file,
    find: "                ty::Ref(_, of, ty::Mutability::Mut) if of == row => row_lent_only_kept(tcx, body, local),\n",
    replace: "                ty::Ref(_, of, ty::Mutability::Mut) if of == row => true,\n",
    tests,
  },
  {
    name: "rows-kept-past-a-slice-call",
    breaks: "a call given the table's slice `&mut`, `board.fill(Vec::new())`, keeps its rows, so `board[0][c]` past it is taken in bounds, a proved-call-bounds line",
    file,
    find: "                    after.rows.shift_remove(&table);\n",
    replace: "",
    tests,
  },
  {
    name: "table-as-long-as-nothing",
    breaks: "`vec![0.0; terraces.len()]` isn't known as long as `terraces`, so `left[i]` under `i < terraces.len()` keeps its check, a call-bounds line",
    file,
    find: "                    Some(of) => after.same_length(local, of),\n",
    replace: "                    Some(_) => {}\n",
    tests,
  },
  {
    name: "table-count-unheld",
    breaks: "`vec![0; count]` isn't known as long as `count`, so `sums[i]` under `i < count` keeps its check, a call-bounds line",
    file,
    find: "                        after.lens.insert(n, local);\n",
    replace: "",
    tests,
  },
  {
    name: "table-rows-unseen",
    breaks: "a table of rows isn't known by its row, so `board[r][c]` under `c < width` keeps its check, a call-bounds line",
    file,
    find: "                after.rows.insert(local, item);\n",
    replace: "",
    tests,
  },
  {
    name: "row-as-long-as-nothing",
    breaks: "`board[r]` isn't known as long as the row the table was made of, so `board[r][c]` under `c < width` keeps its check, a call-bounds line",
    file,
    find: "            after.same_length(local, item);\n",
    replace: "",
    tests,
  },
];
