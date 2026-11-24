// Mutations of crates/rust-js-lower/src/analysis/cell_form/wholes.rs
// (#2490: which `RefCell` fields a body uses whole, `CellForm`'s use rules).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/analysis/cell_form/wholes.rs";
const test = (name: string) => ["test/cell-form-shadow.test.ts", "-t", name];

export const mutations: Mutation[] = [
  {
    name: "cell-form-lent-whole-unseen",
    breaks: "a field lent whole, `&x.f`, to what isn't its own borrow or write isn't a whole use, so its borrows are proved quiet (design point 6)",
    file,
    find: "                if !own {\n",
    replace: "                if false {\n",
    tests: test("is lent whole is counted"),
  },
  {
    name: "cell-form-derive-unseen",
    breaks: "a derive rust-js writes itself, which reads its struct's `RefCell` fields whole, isn't a whole use of them (design point 6, #2479's kind)",
    file,
    find: "                if recognition.is_std_type(ty, StdItem::RefCell) {\n                    let why = format!(\"derived",
    replace: "                if false {\n                    let why = format!(\"derived",
    tests: test("is lent whole is counted"),
  },
  {
    name: "cell-form-param-read-unseen",
    breaks: "a parameter a field is lent to, read otherwise than borrowed, a raw pointer of it to `ptr::eq`, is taken to be only ever borrowed, so the field's whole use is taken for the field's own borrows (#2490's first whole-use proof)",
    file,
    find: "            if mentioned.found && !derived {\n",
    replace: "            if false && mentioned.found && !derived {\n",
    tests: test("is lent whole is counted"),
  },
];
