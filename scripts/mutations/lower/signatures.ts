// Mutations of src/lower/signatures.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "receiver-declared-self",
    breaks: "a method's receiver is declared `self`, not its JS name, `counter` (#507)",
    file: "src/lower/signatures.rs",
    find: "    let name = receiver.unwrap_or_else(|| ident.name.to_string());\n",
    replace: "    let name = ident.name.to_string();\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "parameter-declared-underscored",
    breaks:
      "a parameter's declaration keeps the `_` its JS name drops, `ref` declared `_ref`",
    file: "src/lower/signatures.rs",
    find: "    let named = super::parameter_name(tcx, function, &name);\n",
    replace: "    let named = super::camel_case(&name);\n",
    tests: ["test/lowering-names.test.ts", "-t", "leading underscore"],
  },
  {
    name: "declared-param-rust-name",
    breaks:
      "a parameter named `new` is declared `new`, which TypeScript doesn't read, where the JS names it `new$`",
    file: "src/lower/signatures.rs",
    find: "    crate::names::js_ident(&named)\n}\n",
    replace: "    named\n}\n",
    tests: ["test/declarations.test.ts", "-t", "as its JS does"],
  },
  {
    name: "trailing-option-param-required",
    breaks:
      "a Rust function's trailing `Option` parameter is declared required, `shown(name: string | null | undefined)`, so a TypeScript caller must give it",
    file: "src/lower/signatures.rs",
    find: "                        optional: i >= optional_from,\n",
    replace: "                        optional: false,\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations take JS's null for an Option",
    ],
  },
  {
    name: "nullable-result-declared-undefined",
    breaks:
      "a nullable function's result is declared `string | null | undefined`, though its `None` is `null`, never `undefined`",
    file: "src/lower/signatures.rs",
    find: "            (false, true) if is_option(tcx, output) => match output.kind() {\n",
    replace: "            (false, true) if false => match output.kind() {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "a nullable function's result is declared T or null",
    ],
  },
  {
    name: "nullable-param-declared-optional",
    breaks:
      "a `#[rust_js::nullable(..)]` parameter is declared `fiber?: Fiber | null | undefined`, though it's taken as `Fiber | null` (ADR 0861, amended)",
    file: "src/lower/signatures.rs",
    find: "                let form = match (rest == Some(i), nullable(i)) {\n",
    replace: "                let form = match (rest == Some(i), false) {\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "nullable thread-local's None",
    ],
  },
  {
    name: "optional-from-of-trailing-all",
    breaks: "a parameter `Option` before one that isn't is declared optional, `(value?: number | null, value2: number)`, which TypeScript refuses",
    file: "src/lower/signatures.rs",
    find: "            .take_while(|&(i, &ty)| is_option(tcx, ty) && !nullable(i))\n",
    replace: "            .filter(|&(i, &ty)| is_option(tcx, ty) && !nullable(i))\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "ungiven-fact-declared",
    breaks: "a function whose JS takes a fact of its type parameter, `size(TSize)`, is declared `size<T>(): number`, which gives a JS caller `undefined` (#549)",
    file: "src/lower/signatures.rs",
    find: "        if let Some((index, fact)) = fact {\n",
    replace: "        if let Some((index, fact)) = fact.filter(|_| false) {\n",
    tests: ["test/declarations-ungiven.test.ts"],
  },
  {
    name: "ungiven-dictionary-declared",
    breaks: "a function whose JS takes a bound's dictionary of a trait that isn't declared, `shown(value, TDisplay)`, is declared, which throws for a JS caller that gives none (#549)",
    file: "src/lower/signatures.rs",
    find: "            Form::Dictionary(bound) if !declared(bound.def_id) => {\n",
    replace: "            Form::Dictionary(bound) if false && !declared(bound.def_id) => {\n",
    tests: ["test/declarations-ungiven.test.ts"],
  },
  {
    name: "optional-before-dictionary",
    breaks: "a trailing `Option` before a dictionary is declared optional, `prefix?: string | null`, so a TypeScript caller that leaves it out gives its dictionary as `prefix` (#550)",
    file: "src/lower/signatures.rs",
    find: "            true => inputs.len(),\n",
    replace: "            true => optional_from(tcx, inputs, rest, nullable),\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "trait-method-declared-its-dictionary",
    breaks: "a declared trait's method is declared taking its own trait's dictionary after its arguments, `area(self: Self, SelfShape: Shape<Self>)`, which its dictionary's entry doesn't (#550)",
    file: "src/lower/signatures.rs",
    find: "        let slots = match own && !super::bindings::is_binding(tcx, function) {\n",
    replace: "        let slots = match !super::bindings::is_binding(tcx, function) {\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "caller-location-undeclared",
    breaks: "a `#[track_caller]` function's caller's location, which its JS takes after its arguments, isn't declared, so a TypeScript caller gives none and `checked(0)` throws a `TypeError` where Rust panics naming its caller (#560)",
    file: "src/lower/signatures.rs",
    find: "        for slot in slots {\n",
    replace: "        for slot in slots.into_iter().filter(|slot| !matches!(slot, Slot::Location)) {\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "location and a const"],
  },
  {
    name: "const-value-undeclared",
    breaks: "a const parameter's value, `zeros(N)`'s, isn't declared, so a TypeScript caller gives none and `zeros()` is `[0]` where `zeros::<3>()` is `[0, 0, 0]` (#560)",
    file: "src/lower/signatures.rs",
    find: "        for slot in slots {\n",
    replace: "        for slot in slots.into_iter().filter(|slot| !matches!(slot, Slot::Const(_))) {\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "location and a const"],
  },
  {
    name: "this-receiver-declared",
    breaks:
      "a class's method is declared of its `self`, `setState(component: Component, partialState?: unknown)`, which its JS takes as `this` (ADR 0761, 1091)",
    file: "src/lower/signatures.rs",
    find: "        let skipped = usize::from(super::classes::receiver(tcx, function) != Receiver::First && !inputs.is_empty());\n",
    replace: "        let skipped = usize::from(false && !inputs.is_empty());\n",
    tests: ["test/classes.test.ts"],
  },
  {
    name: "fn-bound-not-callable",
    breaks:
      "a type parameter bound by `Fn`, `twice<F: Fn(u32) -> u32>(f: F)`, is declared a plain `F`, which any value meets, where its JS calls it (#661)",
    file: "src/lower/signatures.rs",
    find: "                || !self.is_callable(bound.self_ty())\n",
    replace: "                || !self.is_callable(bound.self_ty())\n                || !bound.self_ty().to_string().starts_with(\"impl \")\n",
    tests: ["test/declarations-impl-fn.test.ts", "-t", "bound by Fn"],
  },
];
