// Mutations of src/lower/mir/patterns/null_nones.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "null-nones-parameter-taken",
    breaks:
      "a parameter the body sets again by a call, `reason = some(\"a\")`, given on to a nullable parameter, is taken as `null` for `None` at each write, its caller's `undefined` forgotten, so `commit` tests it `!== null` and reads `undefined`'s `length` (#1248)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "            if (1..=body.arg_count).contains(&local.as_usize()) {\n",
    replace: "            if false && (1..=body.arg_count).contains(&local.as_usize()) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_param_given_a_parameter_set_again.rs$"],
  },
  {
    name: "tested-some-copy-unseen",
    breaks:
      "a `let` of `js::null()` set to a place just tested `is_some()` may be `undefined`, so the nullable parameter it's given is tested `!= null` (rd623)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                made.entry(assign.0.local).or_default().0 += 1;\n            }\n            if let Rvalue::Use",
    replace: "            }\n            if let Rvalue::Use",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a let of null set only to a place tested is_some"],
  },
  {
    name: "tested-some-of-any-test",
    breaks: "a place tested `is_none()`, or another place tested, is taken for a `Some` where it's read: `undefined` given as `null` (rd623)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                self.recognition().classify(def_id, generic_args) == Some(Std::IsSome)\n            })\n            && reference.is_some_and(",
    replace: "                matches!(self.recognition().classify(def_id, generic_args), Some(Std::IsSome | Std::IsNone))\n            })\n            && reference.is_some_and(",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a let of null set only to a place tested is_some"],
  },
  {
    name: "tested-some-of-another-place",
    breaks: "a copy of one place where another was tested `is_some()` is taken for a `Some`: `undefined` given as `null` (rd623)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                            && matches!(&assign.1, Rvalue::Ref(_, BorrowKind::Shared, of) if *of == place))",
    replace: "                            && matches!(&assign.1, Rvalue::Ref(_, BorrowKind::Shared, _)))",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a let of null set only to a place tested is_some"],
  },
  {
    name: "null-call-called",
    breaks: "`js::null()`'s `None`, which is `null`, is taken for a call's that may be `undefined` (rd623)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                if !null {\n                    called.insert(destination.local);\n                }\n",
    replace: "                let _ = null;\n                called.insert(destination.local);\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a let of null set only to a place tested is_some"],
  },
  {
    name: "null-none-copy-ungiven",
    breaks:
      "`let owner = None;` given to a nullable field is `undefined` where it's made, and `owner ?? null` where it's given (ADR 0781)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                    0 => *given.entry(of).or_default() += 1,\n",
    replace: "                    0 => {}\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a None only nullable fields and parameters read",
    ],
  },
  {
    name: "null-none-given-optional",
    breaks:
      "`let label = None;` given to an optional nullable field is `undefined` where it's made, and `label ?? null` where it's given (ADR 0810, amended)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                    && none_form(self.tcx, NonePlace::Field(&fields[field], None)).stores_null()\n",
    replace: "                    && none_form(self.tcx, NonePlace::Field(&fields[field], None)) == NoneForm::Null\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "an optional nullable field",
    ],
  },
  {
    name: "null-none-param-ungiven",
    breaks:
      "`let owner = None;` given to a binding's nullable parameter is `undefined` where it's made (ADR 0781)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                    !nullable.is_empty() && none_form(self.tcx, NonePlace::BindingArg(def_id, i, None)).stores_null();\n",
    replace: "                    !nullable.is_empty() && none_form(self.tcx, NonePlace::BindingArg(def_id, i, None)).stores_null() && false;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a None only nullable fields and parameters read",
    ],
  },
  {
    name: "null-none-copy-read-elsewhere",
    breaks:
      "a `None` copied to a variable read elsewhere is `null`, so `let other = owner;` returns `null`, not `undefined` (ADR 0781)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "            if locals.writes[copy] == 1 && to + alike_to == locals.reads[copy] && to + alike_to > 0 {\n",
    replace: "            if locals.writes[copy] == 1 {\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a None only nullable fields and parameters read",
    ],
  },
  {
    name: "nullable-result-some-coalesced",
    breaks:
      "a `#[rust_js::nullable]` function's `Some(x)` is `x ?? null`, where it's made a value and is one",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "            if local == mir::RETURN_PLACE && result {\n",
    replace: "            if local == mir::RETURN_PLACE && result && false {\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "a nullable function's None is null",
    ],
  },
  {
    name: "null-nones-calls-unwritten",
    breaks:
      "a call's `Option` only a nullable field reads is `?? null` where it's read, not `null` where it's made",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                *none = true;\n",
    replace: "                *none = true;\n                *writes += 1;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a nullable function's result is written to a nullable place as it is",
    ],
  },
  {
    name: "null-nones-own-param-call-coalesced",
    breaks:
      "a call's `Option` only a function's own nullable parameter reads is written `?? null`, `mapIntoArray(init(payload) ?? null, ..)`, where its body reads either (ADR 0861)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "            if !(called && !stored.contains(&local)) {\n",
    replace: "            if true {\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "own nullable parameter"],
  },
  {
    name: "nullable-items-made-undefined",
    breaks:
      "a `None` given where a nullable field's items are made, `create_lane_map(None)`, is `undefined`, not upstream's `createLaneMap(null)`",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "            *given.entry(option).or_default() += 1;\n",
    replace: "            let _ = option;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "Vec of Options holds null",
    ],
  },
  {
    name: "nullable-init-given-undefined",
    breaks:
      "a `#[rust_js::nullable]` thread-local's generic constructor is given `undefined` for `None`, `createCursor(undefined)`, not upstream's `createCursor(null)` (ADR 0586, amended)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                    made && inputs.get(i).is_some_and(|input| matches!(input.kind(), ty::Param(_)));\n",
    replace:
      "                    made && inputs.get(i).is_some_and(|_| false);\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a generic constructor makes",
    ],
  },
  {
    name: "nullable-init-gives-any-parameter",
    breaks:
      "a `#[rust_js::nullable]` thread-local's constructor is given `null` for an `Option` its own parameter's type is, which isn't a type parameter's value, `pair(null, null)` (ADR 0586, amended)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                    made && inputs.get(i).is_some_and(|input| matches!(input.kind(), ty::Param(_)));\n",
    replace: "                    made && inputs.get(i).is_some();\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a generic constructor makes",
    ],
  },
  {
    name: "nullable-init-gives-any-call",
    breaks:
      "a call a `#[rust_js::nullable]` thread-local's initializer makes on the way, not the one whose value it is, is given `null` for `None`, `id(null)` (ADR 0586, amended)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "            let made = destination.local == mir::RETURN_PLACE\n",
    replace: "            let made = true\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a generic constructor makes",
    ],
  },
  {
    name: "nullable-return-not-a-nullable-read",
    breaks:
      "a `let` of `None` a nullable function returns isn't read by a nullable place, so it starts `undefined` and is returned `?? null`",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                if returns_nullable && assign.0.local == mir::RETURN_PLACE {\n",
    replace:
      "                if false && returns_nullable && assign.0.local == mir::RETURN_PLACE {\n",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "-t",
      "a nullable parameter, a nullable thread-local",
    ],
  },
  {
    name: "some-in-a-temporary-not-a-write",
    breaks:
      "`found = Some(Rc::clone(&n))`, made in a temporary MIR moves in, isn't a write of a `Some`, so `found`'s `None` stays `undefined`",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                made.entry(copy).or_default().0 += 1;\n",
    replace: "",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "-t",
      "a nullable parameter, a nullable thread-local",
    ],
  },
  {
    name: "let-alike-of-a-temporary",
    breaks: "a temporary a call writes, given straight to a nullable parameter, `commitLoose(profiling ? reason() : undefined)`, is `null` for `None` too, a `?? null` where no `let` is (rd632, the user's (B) is of a `let`)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                && lets.contains(&local)\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "returns null or a value"],
  },
  {
    name: "let-read-otherwise-nulled",
    breaks: "a `let` a call writes, read by a nullable parameter and any other way, a binding's `String(previous)` say, is `null` for `None`, so `String` gives \"null\" where Rust's gives \"undefined\" (rd632)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                && given + alike == locals.reads[local]\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "read by a nullable parameter and alike"],
  },
  {
    name: "null-nones-clones-written",
    breaks:
      "a clone of an optional nullable field, which may be left out, is written `?? null` where it's made, so another optional field given it stores `null` where it should store `undefined` (ADR 0810, amended; a nullable field's clone is given as it is either way since rd632's `call_nulled`)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "            if !cloned\n",
    replace: "            if true\n",
    tests: ["test/lowering-optional-fields.test.ts", "-t", "an optional nullable field given another's value stores it as it is"],
  },
  {
    name: "let-arms-uncounted",
    breaks: "a `let` whose arm is a `let` of its own, `let previous = previous_of(..); previous`, has a write that isn't what it makes, so it's never `null` for `None` and `measure(previous)` stays `!= null` (rd632)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                && writes + arms.len() - moved == locals.writes[local]\n",
    replace: "                && writes - moved == locals.writes[local]\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "read by a nullable parameter and alike"],
  },
  {
    name: "let-arms-unnulled",
    breaks: "a `let`'s arm that's a `let` of its own, read before a cell it read is cleared, `let previous = fiber.memoized.get()..; fiber.memoized.set(None); previous`, isn't `null` for `None` where it's written, so its copy is `previous = previous$1 ?? null` (rd632)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                nulled.extend(arms);\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "read by a nullable parameter and alike"],
  },
  {
    name: "let-arm-some-counted-twice",
    breaks:
      "a `Some` an arm moves into a late `let` is counted as its write twice, so the `let`'s `None` is `undefined`, `previousMeasurements = undefined` (rd647)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                && writes + arms.len() - moved == locals.writes[local]\n",
    replace: "                && writes + arms.len() == locals.writes[local]\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_let_set_in_arms.rs$"],
    snapshots: true,
  },
  {
    name: "let-arms-counted-each-copy",
    breaks:
      "an arm a `let` is copied from twice in MIR's statements is counted as two writes, so the `let`'s `None` is `undefined` (rd647)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "            let arms: std::collections::BTreeSet<Local> = (copies.iter())\n",
    replace: "            let arms: Vec<Local> = (copies.iter())\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_let_set_in_arms.rs$"],
    snapshots: true,
  },
  {
    name: "null-nones-local-set-unread",
    breaks:
      "an `Option` a nullable thread-local's `set` alone reads keeps its `None` `undefined` where it's made: React's ReactFlightServer printed `importStringRequest = __DEV__ && debug ? undefined : request;`, where upstream writes `null` (#683, ADR 0861, amended)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                    .is_some_and(|key| none_form(self.tcx, NonePlace::Local(key)).stores_null())\n            {\n                *given.entry(read.local).or_default() += 1;\n",
    replace: "                    .is_some_and(|key| none_form(self.tcx, NonePlace::Local(key)).stores_null())\n            {\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "stores a test tidy makes of its value"],
  },
  {
    name: "null-nones-any-local-set",
    breaks:
      "an `Option` any thread-local's `set` alone reads is `null` for `None` where it's made, though one not `#[rust_js::nullable]` holds `undefined` for `None` (#683, ADR 0861, amended)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                    .is_some_and(|key| none_form(self.tcx, NonePlace::Local(key)).stores_null())\n            {\n",
    replace: "                    .is_some()\n            {\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "stores a test tidy makes of its value"],
  },
  {
    name: "nullable-init-of-any-thread-local",
    breaks:
      "a thread-local that isn't `#[rust_js::nullable]`, made by a generic constructor, is given `null` for `None`, `createCursor(null)` where its JS reads `undefined` (ADR 0586, amended)",
    file: "src/lower/mir/patterns/null_nones.rs",
    find: "                    .is_some_and(|key| none_form(self.tcx, NonePlace::Local(key)).stores_null());\n",
    replace: "                    .is_some();\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a generic constructor makes",
    ],
  },
];
