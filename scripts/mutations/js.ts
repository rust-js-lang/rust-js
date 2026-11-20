// Mutations of src/js.rs.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "declared-name-of-none",
    breaks: "a `const` declares no variable another item reads, so a thread-local's `const` read by one made before it is made after it, which JS throws on (ADR 0306)",
    file: "src/js.rs",
    find: "            Item::Namespace(Namespace { name, .. }) | Item::Const(Const { name, .. }) => Some(name),\n",
    replace: "            Item::Namespace(_) | Item::Const(_) => None,\n",
    tests: ["test/modules.test.ts", "-t", "in source order, each after what it reads"],
  },
  {
    name: "load-statements-unread",
    breaks: "the helpers what a module runs when it's loaded reads, `$index`, aren't imported",
    file: "src/js.rs",
    find: "            Item::Statements(stmts) => visit_stmt_names(stmts, read),\n",
    replace: "            Item::Statements(_) => {}\n",
    tests: ["test/modules.test.ts", "-t", "js::on_load!"],
  },
  {
    name: "option-eq-one-side-none",
    breaks: "a getter's `T | null` compared with the crate's `Option`, maybe `undefined`, by `===`, so `a.parentNode === b` is false of `null` and `None` (rd117)",
    file: "src/js.rs",
    find: "        self.only_none().is_some() && self.only_none() == other.only_none()\n",
    replace: "        self.only_none().is_some()\n",
    tests: ["test/webapi-bindings.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "evaluation-facts-forgotten",
    breaks: "an operator of any value made again, `+m` or `m != 1` of `!(m == 1)`, forgets that JS may throw of it, so unread it goes (#462)",
    file: "src/js.rs",
    find: "        self.part(&[Known::Getter])\n",
    replace: "        Facts::NONE\n",
    tests: ["test/lowering-unwind-arms.test.ts", "-t", "a caught value nothing reads that may throw"],
  },
  {
    name: "fact-forgotten-still-found",
    breaks: "a fact `--without-known` forgets is still found by its readers, so the net's variants each try the JS a build makes, and none without the fact (ADR 1127)",
    file: "src/js.rs",
    find: "        let has = known::seen(self.0) & known.bit();\n",
    replace: "        let has = self.0 & known.bit();\n",
    tests: ["test/tidy-stages.test.ts", "-t", "a case's JS without a fact a reader found"],
  },
  {
    name: "found-what-is-forgotten",
    breaks: "`--knowns-found` names a fact the compile forgot, which no reader found: a variant is made of a fact that changes nothing (ADR 1127)",
    file: "src/js.rs",
    find: "        known::found(has);\n",
    replace: "        known::found(self.0 & known.bit());\n",
    tests: ["test/tidy-stages.test.ts", "-t", "a case's JS without a fact a reader found"],
  },
  {
    name: "strict-presence-no-test-of-being-set",
    breaks: "a strict test of an `Option`'s being set, `x !== undefined`, isn't one to the passes that fold, merge and move such tests, so what they made of the loose one they no longer make (#2112 C S5)",
    file: "src/js.rs",
    find: "            Op::LooseNe | Op::NoneNe(_) => Some(true),\n",
    replace: "            Op::LooseNe => Some(true),\n",
    tests: ["test/lowering-written-branches.test.ts", "test/lowering-bindings.test.ts", "test/none-given.test.ts"],
  },
  {
    name: "program-strict-equality-a-test-of-being-set",
    breaks: "a program's own `js::strict_equals`, `x !== null`, is taken for a test of an `Option`'s being set, so its ternary folds into `x?.a`, which of an `undefined` `x` gives `undefined` where `x.a` throws (#2112 C S5)",
    file: "src/js.rs",
    find: "            Op::LooseEq | Op::NoneEq(_) => Some(false),\n",
    replace: "            Op::LooseEq | Op::NoneEq(_) | Op::Eq => Some(false),\n            Op::Ne => Some(true),\n",
    tests: ["test/lowering-nullable-tests.test.ts", "-t", "strict_equals ternary never folds"],
  },
];
