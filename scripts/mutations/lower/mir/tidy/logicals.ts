// Mutations of src/lower/mir/tidy/logicals.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "implied-part-kept",
    breaks: "`x != null && typeof x === \"number\"` keeps the test the later one implies",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "        *test = unimplied(Expr::bin(Op::And, first, second));\n",
    replace: "        *test = Expr::bin(Op::And, first, second);\n",
    tests: ["test/lowering-matches.test.ts", "-t", "a match arm's pattern of parts"],
  },
  {
    name: "mir-tidy-and-tests-any-condition",
    breaks: "`if`s the program nests are merged into one `&&`",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "        let one = (conditions.iter()).any(|(first, then)| within(first, span) && within(then, inner.span));\n",
    replace: "        let one = true;\n",
    tests: ["test/lowering-loops.test.ts", "-t", "if-let chains and loops from MIR"],
  },
  {
    "name": "and-test-bound-kept",
    "breaks": "a test merged into `a && b` leaves its bound read, `const captured = child.type`, read again and unread",
    "file": "src/lower/mir/tidy/logicals.rs",
    "find": "                && js::mentions_in(&then[at + 1..], name) == 0\n",
    "replace": "                && false\n",
    "tests": ["test/jsx-children.test.ts", "-t", "capture of an inlined closure"]
  },
  {
    name: "logical-block-kept",
    breaks: "`(x == y && (x != 0.0 || ..)) || ..` is a labelled block of its tests",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "    let Some(made) = value(body, after, (label, after), returns, booleans) else {\n",
    replace: "    let Some(made) = None::<Expr> else {\n",
    tests: ["test/lowering-branches.test.ts", "-t", "a value of && and"],
  },
  {
    name: "logical-block-unfactored",
    breaks: "`c ? (p || d) : d` isn't `(c && p) || d`, so the block stays",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "    if let Some(p) = without(&yes, &no) {\n",
    replace: "    if let Some(p) = without(&yes, &no).filter(|_| false) {\n",
    tests: ["test/lowering-branches.test.ts", "-t", "a value of && and"],
  },
  {
    "name": "logical-returns-kept",
    "breaks": "a value written with `&&`, `s != \"en\" && (if t {..} else {..})`, stays MIR's `return` of each branch",
    "file": "src/lower/mir/tidy/logicals.rs",
    "find": "        && within(last_if.span)\n",
    "replace": "        && false\n",
    "tests": ["test/lowering-branches.test.ts", "-t", "a value written with && is returned as one"]
  },
  {
    name: "bound-property-read-twice",
    breaks: "a property a `let` chain binds and tests is read in its test and again for its binding, `typeof request.url === \"string\"` then `const url = request.url`: a getter runs twice, where `js::get` runs once",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "            if getter(value) && tested > 0 && tested + again(at, name) > 1 {\n",
    replace: "            if false && getter(value) && tested > 0 && tested + again(at, name) > 1 {\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "a property a let chain tests and binds is read once"],
  },
  {
    name: "bound-rust-data-a-getter",
    breaks: "Rust's data a chain binds and tests, a field, `const c = node.count`, or a tuple's item, `value[0]`, isn't read in the test, `node.count != null && node.count > 1`, as if a getter could read it",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "            && (getters.iter()).any(|at| at.lo <= e.span.lo && e.span.hi <= at.hi)\n",
    replace: "            || true\n",
    tests: ["test/lowering-loops.test.ts", "-t", "if-let chains and loops from MIR read as written"],
  },

];
