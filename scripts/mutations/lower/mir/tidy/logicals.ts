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
    "find": "            && js::mentions_in(&stmts[at + 1..], name) == 0\n",
    "replace": "            && false\n",
    "tests": ["test/corpus.test.ts", "-t", "^(signed_index|signed_counter).rs$"],
    "snapshots": true
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
    find: "    e.known == js::Known::Getter\n",
    replace: "    true\n",
    tests: ["test/lowering-loops.test.ts", "-t", "if-let chains and loops from MIR read as written"],
  },

  {
    name: "read-once-not-first",
    breaks: "a chain whose property is read once, its failures alike, reaches its `else` by a label, `bb27: { .. break bb27; }`, not `const inner = isValidElement(children) ? children.props.children : undefined;`",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "            first_reads.extend(read_once_first(stmt).map(|read| (i, read)));\n",
    replace: "            first_reads.extend(read_once_first(stmt).map(|read| (i, read)).filter(|_| false));\n",
    tests: ["test/lowering-bindings.test.ts", "test/jsx-types.test.ts", "-t", "a property a let chain tests and binds is read once|let chain as a person"],
  },
  {
    name: "read-once-first-of-any-chain",
    breaks: "a chain whose property is read once and that returns, with no label to reach, reads it first anyway, `const url = request != null && .. ? request.url : undefined`",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "    if !getter || others || second.has_effects() || !breaks || !tests.iter().any(|t| implies(t, &defined)) {\n",
    replace: "    if !getter || others || second.has_effects() || !tests.iter().any(|t| implies(t, &defined)) {\n",
    tests: ["test/lowering-switches.test.ts", "-t", "React's getIODescription"],
  },

  {
    name: "read-once-first-of-a-call",
    breaks: "a call a chain binds, `const q = counted(h)`, is made first as a conditional, `const q = h != null ? .. counted(h) : undefined`, though nothing reads it twice",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "    let getter = reads_a_getter(value);\n",
    replace: "    let getter = true;\n",
    tests: ["test/snapshots.test.ts", "-t", "options"],
    snapshots: true,
  },
  {
    name: "or-block-negated",
    breaks: "an `||` the program wrote whose branch is a `break` is negated, `if (!a && !b) { r } s`, its `break` after (ADR 0790)",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "        if let Some((test, span, rest)) = made {",
    replace: "        if let Some((test, span, rest)) = made.filter(|_| false) {",
    tests: ["test/lowering-labels.test.ts", "-t", "a block the program labeled that ends a case"],
  },
];
