// Mutations of src/lower/serde/std_forms.rs, the catalog of std types'
// serde forms (#1623).
import type { Mutation } from "../../../mutations";

const ranges = ["test/serde-std.test.ts", "-t", "a range and a bound"];

export const mutations: Mutation[] = [
  {
    name: "std-range-row-missing",
    breaks: "a `Range` has no row, so writing or reading one is refused where serde writes `{\"start\",\"end\"}` (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: '            Some(RangeKind::Exclusive) => return range("Range"),\n',
    replace: "",
    tests: ranges,
  },
  {
    name: "std-struct-form-unknown-field-kept",
    breaks: "a std struct form ignores a field it doesn't know, where serde's own visitor refuses it, `unknown field `step`, expected `start` or `end`` (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: '                    js::Prop::Field("deny".into(), Expr::bool(true)),\n',
    replace: "",
    tests: ranges,
  },
  {
    name: "std-struct-form-derive-words",
    breaks: "a std struct form's sequence too short says a derive's words, `struct Range with 2 elements`, where serde's visitor says `struct Range` (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: '                    js::Prop::Field("expecting".into(), expected.clone()),\n',
    replace: "",
    tests: ranges,
  },
  {
    name: "std-enum-form-declared-order",
    breaks: "`Bound`'s variants are named in the enum's declared order, `Included`, `Excluded`, `Unbounded`, where serde's message lists `Unbounded`, `Included`, `Excluded` (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: '&["Unbounded", "Included", "Excluded"]',
    replace: '&["Included", "Excluded", "Unbounded"]',
    tests: ranges,
  },
];
