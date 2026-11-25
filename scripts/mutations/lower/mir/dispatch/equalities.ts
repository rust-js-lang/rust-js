// Mutations of crates/rust-js-mir/src/dispatch/equalities.rs (ADR 0093):
// `==` of `Option`s of `Option`s and whether an `==` is `===`, split from
// dispatch.rs by concept (#1334).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-mir/src/dispatch/equalities.rs";

export const mutations: Mutation[] = [
  {
    name: "nested-flatten-unread",
    breaks: "`flatten()` of a nested field is `$someValue(..)` of its box, not `entry.value` (#470)",
    file,
    find: "            && self.recognition().classify(def_id, generic_args) == Some(Std::OptionFlatten)\n        {\n            let place = x.place().expect(\"a place\");\n",
    replace: "            && self.recognition().classify(def_id, generic_args) == Some(Std::OptionFlatten)\n            && false\n        {\n            let place = x.place().expect(\"a place\");\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "some-equality-of-any",
    breaks:
      "an `==` of two `Option`s, either of which may be `null` or `undefined`, is `===`",
    file,
    find: "                place.projection.is_empty() && state.facts.options().made_some.contains(&place.local)\n",
    replace:
      "                place.projection.is_empty() || state.facts.options().made_some.contains(&place.local)\n",
    tests: ["test/lowering-options.test.ts", "-t", "compared with a Some"],
  },
  {
    name: "registered-equality-loose",
    breaks:
      '`from_value(x) == Some(SymbolFor("k"))` is `x == Symbol.for("k")`, true of an object that turns into the symbol, which isn\'t one',
    file,
    find: "            .is_some_and(|equality| self.frame.facts.registered_equalities.contains(&equality)))\n",
    replace: "            .is_some_and(|equality| false && self.frame.facts.registered_equalities.contains(&equality)))\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "an == of a value read as the registry's symbol",
    ],
  },
  {
    name: "nested-option-eq-loose",
    breaks:
      "two held `Option`s of `Option`s are compared `==`, so `None` and `Some(None)` are equal (ADR 1017)",
    file,
    find: "            Expr::bin(Op::Eq, a, b)\n        } else {\n",
    replace: "            Expr::bin(Op::LooseEq, a, b)\n        } else {\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-eq-nones-unseen",
    breaks:
      "two held `Option`s of `Option`s of strings are compared as `Option<String>`s, so `None` and `Some(None)` are equal (ADR 1017)",
    file,
    find: "            Expr::cond(none, Expr::bin(Op::Eq, a, b), both)\n",
    replace: "            { let _ = (none, a, b); both }\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-box-unpeeled",
    breaks:
      "two held `Option`s of `Option<Box<u32>>`s are compared through `$eq`, where `===` is their `==` (ADR 1017)",
    file,
    find: "            && (held.is_box() || self.is_rc(held) && self.counted_rc(held).is_none())\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-is-some-flipped",
    breaks:
      "`is_some()` of an `Option` of an `Option` held as its payload is `last === undefined`, a wrong answer",
    file,
    find: "            let op = if known == Std::IsSome { Op::Ne } else { Op::Eq };\n",
    replace:
      "            let op = if known == Std::IsSome { Op::Eq } else { Op::Ne };\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
  },
  {
    name: "nested-option-eq-some-none-undefined",
    breaks:
      "`last == Some(None)` of an `Option` of an `Option` held as its payload is `last === undefined`, its `None`'s test, a wrong answer",
    file,
    find: "            return Ok(Some(Expr::bin(op, x, Expr::null())));\n",
    replace:
      "            return Ok(Some(Expr::bin(op, x, Expr::js_undefined())));\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
  },
  {
    name: "held-is-some-undefined",
    breaks:
      "`is_some()` of a nullable field of what may be `undefined`, through a reference, is `!== undefined`, its `Some` a `None` (rd645)",
    file,
    find: "            let none = self.held_none(x.ty(state.body, self.tcx));\n",
    replace: "            let none = Expr::js_undefined();\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_of_undefined.rs$"],
  },
];
