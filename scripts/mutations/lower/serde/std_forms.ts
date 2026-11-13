// Mutations of src/lower/serde/std_forms.rs, the catalog of std types'
// serde forms (#1623).
import type { Mutation } from "../../../mutations";

const ranges = ["test/serde-std.test.ts", "-t", "a range and a bound"];
const time = ["test/serde-std.test.ts", "-t", "a Duration and a SystemTime"];
const wrappers = ["test/serde-std.test.ts", "-t", "a Wrapping, a Reverse and a PhantomData"];

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
  {
    name: "std-wrapper-row-missing",
    breaks: "a `Reverse` or a `Wrapping` has no row, so writing or reading one is refused where serde writes what it holds (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: "        if self.is_reverse(ty) || self.is_std_type(ty, StdItem::Wrapping) {\n",
    replace: "        if false {\n",
    tests: wrappers,
  },
  {
    name: "std-newtype-written-as-its-array",
    breaks: "a `Reverse` or a `Wrapping` is written as the array that holds it, `[3]`, where serde writes what it holds, `3` (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: "self.write_json(Expr::index(value, Expr::int(0)), json, inner, span, out)",
    replace: "self.write_json(value, json, Ty::new_tup(self.tcx, &[inner]), span, out)",
    tests: wrappers,
  },
  {
    name: "std-newtype-read-unwrapped",
    breaks: "a `Reverse` or a `Wrapping` read is what it holds, not its tuple struct's array, so reading its field gives `undefined` (#1623)",
    file: "src/lower/serde/std_forms.rs",
    find: "            let made = Expr::array(vec![Expr::call(read, vec![Expr::var(&json)])]);\n",
    replace: "            let made = Expr::call(read, vec![Expr::var(&json)]);\n",
    tests: wrappers,
  },
];
