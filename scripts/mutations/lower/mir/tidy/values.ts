// Mutations of src/lower/mir/tidy/values.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-plain-cell-escapes",
    breaks: "a cell given whole, `bump(c)`, is a variable too, and what it's given reads `undefined`",
    file: "src/lower/mir/tidy/values.rs",
    find: "    let plain: HashSet<&String> = (cells.iter()).filter(|c| reads.get(*c) == kept.get(*c)).collect();\n",
    replace: "    let plain: HashSet<&String> = cells.iter().collect();\n",
    tests: ["test/lowering.test.ts","-t","only its function reads and sets"],
  },
  {
    "name": "mir-tidy-passed-on-unread",
    "breaks": "an array method's callback from MIR that passes on what it's given reads `xs.map(parseFloat)`, a JS function's, which may read the index",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "        && forwarding.contains(&f.span)\n",
    "replace": "        && true\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "only passes its arguments on"
    ]
  },
  {
    "name": "array-spread-of-array-kept",
    "breaks": "`[...first, ...[a, b]]` keeps its spread of an array, not `[...first, a, b]`",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "    if let ExprKind::Array(items) = &mut e.kind {",
    "replace": "    if let ExprKind::Array(items) = &mut e.kind && false {",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arrays joined are one array of their items"
    ]
  },
  {
    "name": "array-values-kept-stepped",
    "breaks": "an array's `$iter` an adapter alone steps stays `$iter(xs).filter(..)`, not `xs.values().filter(..)`",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "        && crate::lower::iterators::is_stepped_items(it)\n        && let ExprKind::Call(_, args) = &mut it.kind\n",
    "replace": "        && false\n        && let ExprKind::Call(_, args) = &mut it.kind\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an array's iterator for an adapter alone is its values"
    ]
  },
  {
    "name": "named-callback-reads-base",
    "breaks": "a closure reading what it would be named, `step` of `step$1`, is named it, reading itself",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "            !reads\n",
    "replace": "            true\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-enumerate-mapped",
    "breaks": "`iter().enumerate()` of a slice from MIR is `xs.values().map((x, i) => [i, x])`, not `xs.entries()`",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "        && pairs_index(pair)\n",
    "replace": "        && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "nested loops from MIR read as written"
    ]
  },
  {
    "name": "mir-rest-str-of-fresh",
    "breaks": "`w.chars().as_str()` from MIR is `$restStr($iter(Array.from(w)))`, not `Array.from(w).join(\"\")`",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "        && callee.helper_name() == Some(\"$restStr\")\n",
    "replace": "        && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "the rest of a string's chars from MIR"
    ]
  },
  {
    "name": "mir-capital-chars-joined",
    "breaks": "`c.to_uppercase().collect::<String>()` from MIR is `Array.from(c.toUpperCase()).join(\"\")`",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "if m == \"toUpperCase\" || m == \"toLowerCase\"))",
    "replace": "if m == \"none\"))",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "mir-set-of-iter",
    "breaks": "`[1, 2].into_iter().collect::<HashSet<_>>()` from MIR is `new Set($iter([1, 2]))`",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "if c == \"Set\" || c == \"Map\")",
    "replace": "if c == \"None\")",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "sets and maps from MIR"
    ]
  },
  {
    name: "mir-tidy-plain-cell-shadowed",
    breaks: "a cell a callback's parameter shadows is made plain, `(item) => item.value` read as `item`",
    file: "src/lower/mir/tidy/values.rs",
    find: "    cells.retain(|c| bound.get(c).copied().unwrap_or(0) <= 1);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "callback's parameter shadows"],
  },
  {
    name: "mir-tidy-plain-cells-not-in-closures",
    breaks: "MIR's cell cloned in a closure stays a `{ value }`",
    file: "src/lower/mir/tidy/values.rs",
    find: "    let mut clones: HashMap<String, String> = HashMap::new();\n    js::each_block_mut(&mut body.clone(), &mut |stmts| {\n",
    replace: "    let mut clones: HashMap<String, String> = HashMap::new();\n    js::statement_lists(&mut body.clone(), &mut |stmts| {\n",
    tests: ["test/lowering.test.ts","-t","a cell cloned in a closure"],
  },
  {
    name: "mir-tidy-handle-read-early",
    breaks: "MIR's handle on a `Cell` field is read where it's made, though set before it's read",
    file: "src/lower/mir/tidy/values.rs",
    find: "                handles.push((name.clone(), (**place).clone(), read_at_once(stmts, k, name)));\n",
    replace: "                handles.push((name.clone(), (**place).clone(), true));\n",
    tests: ["test/lowering.test.ts","-t","a Cell a let takes apart"],
  },
  {
    "name": "lazy-flatmap-stepped",
    "breaks": "a lazy flatten of arrays keeps `$iter(item)`",
    "file": "src/lower/mir/tidy/values.rs",
    "find": "        *each = Expr::var(param);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arrays of arrays flattened"
    ]
  },
  {
    name: "applied-template-nested",
    breaks: "a function's body put in place of its call in a template is a template in a template, `${`(${a}, ${b})`}`",
    file: "src/lower/mir/tidy/values.rs",
    find: "            root.each_mut(&mut spliced_templates);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^const_generic_traits.rs$"],
    snapshots: true,
  },
  {
    name: "applied-assignment-unseen",
    breaks: "`out += `${((r) => ..)(f())}``, an assignment's value, keeps its function called in place",
    file: "src/lower/mir/tidy/values.rs",
    find: "            StmtKind::Assign(target, e) | StmtKind::Compound(_, target, e) => {\n",
    replace: "            StmtKind::Assign(target, e) | StmtKind::Compound(_, target, e) if false => {\n",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
    snapshots: true,
  },
  {
    name: "applied-arrow-kept",
    breaks: "a closure read in place is called where it's made, `((e) => e.data)(r._0)`, not its value",
    file: "src/lower/mir/tidy/values.rs",
    find: "            *e = Expr {\n                span: e.span,\n                ..applied\n            };\n",
    replace: "            let _ = applied;\n",
    tests: ["test/corpus.test.ts", "-t", "^poison_error.rs$"],
    snapshots: true,
  },
];
