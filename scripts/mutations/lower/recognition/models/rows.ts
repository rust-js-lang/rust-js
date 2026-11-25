// Mutations of crates/rust-js-lower/src/recognition/models/rows.rs (ADR 0093):
// the catalog's rows of `$stdSites`, each by the std function whose panics it
// holds (ADR 1184, ADR 1202).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/recognition/models/rows.rs";

export const mutations: Mutation[] = [
  {
    name: "rc-slice-layout-row-of-another",
    breaks: "`rcSliceLayout`'s row walks `Rc`'s `allocate_for_layout`, not `allocate_for_slice`, so its site isn't the `unwrap()` std panics at (#2489)",
    file,
    find: "        StdRow::RcSliceLayout,\n        None,\n        Owner::Type(StdCrate::Alloc, &[\"rc\", \"Rc\"]),\n        \"allocate_for_slice\",\n",
    replace: "        StdRow::RcSliceLayout,\n        None,\n        Owner::Type(StdCrate::Alloc, &[\"rc\", \"Rc\"]),\n        \"allocate_for_layout\",\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "slice-repeat-row-of-another",
    breaks: "`sliceRepeat`'s row walks a slice's `to_vec`, not its `repeat`, so its site isn't the `checked_mul` std panics at (#2488)",
    file,
    find: "        StdRow::SliceRepeat,\n        None,\n        Owner::Primitive(SimplifiedType::Slice),\n        \"repeat\",\n",
    replace: "        StdRow::SliceRepeat,\n        None,\n        Owner::Primitive(SimplifiedType::Slice),\n        \"to_vec_in\",\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "std-row-named-by-another",
    breaks: "a row's handle names another's row, `vecDrain`'s `vecDequeDrain`'s, so `bun run std` writes no `vecDrain` and its reader reads `undefined`",
    file,
    find: '    VecDrain = "vecDrain",\n',
    replace: '    VecDrain = "vecDequeDrain",\n',
    unitTests: "recognition::compiled_std::raised::tests::std_rows_are_those_written",
  },
];
