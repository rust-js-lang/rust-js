// Mutations of src/lower/serde/std_forms.rs, the catalog of std types'
// serde forms (#1623).
import type { Mutation } from "../../../mutations";

const ranges = ["test/serde-std.test.ts", "-t", "a range and a bound"];
const time = ["test/serde-std.test.ts", "-t", "a Duration and a SystemTime"];

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
  {
    name: "std-duration-row-missing",
    breaks: "a `Duration` has no row, so writing or reading one is refused where serde writes `{\"secs\",\"nanos\"}` (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: "        if crate::lower::recognition::is_duration_ty(ty) {\n",
    replace: "        if false && crate::lower::recognition::is_duration_ty(ty) {\n",
    tests: time,
  },
  {
    name: "std-system-time-written-before-epoch",
    breaks: "a `SystemTime` before the epoch is written as a negative offset, where serde's is `SystemTime must be later than UNIX_EPOCH` (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: "                            Some(helper) => Expr::call_helper(helper, vec![value]),\n",
    replace: "                            Some(_) => value,\n",
    tests: time,
  },
  {
    name: "std-system-time-past-unix-read",
    breaks: "a `SystemTime` whose offset unix's `i64` seconds can't hold is read, where serde's is `overflow deserializing SystemTime`, of no position (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: "                    Some(then) => Expr::call_helper(then, vec![read]),\n",
    replace: "                    Some(_) => read,\n",
    tests: time,
  },
];
