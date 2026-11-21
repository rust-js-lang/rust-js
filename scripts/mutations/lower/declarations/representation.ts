// Mutations of src/lower/declarations/representation.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "top-payload-box-imported",
    breaks: "an `Option` of what's declared `any`, a unit struct's, imports `SomeNone` it never prints, `unstable_Profiling: any` beside an unused `import type { SomeNone }` (#842)",
    file: "src/lower/declarations/representation.rs",
    find: "        if generic || top || !(self.representation.boxed)(self.owner, payload) {\n",
    replace: "        if generic || !(self.representation.boxed)(self.owner, payload) {\n",
    tests: ["test/port-scheduler.test.ts"],
  },
  {
    name: "generic-payload-box-declared",
    breaks: "a generic `Option<R::Event>` is declared `R[\"Event\"] | SomeNone`, a box only an instance whose payload looks like `None` has, which the generic declaration can't tell (#842)",
    file: "src/lower/declarations/representation.rs",
    find: "        if generic || top || !(self.representation.boxed)(self.owner, payload) {\n",
    replace: "        if top || !(self.representation.boxed)(self.owner, payload) {\n",
    tests: ["test/declarations-type-maps.test.ts", "-t", "its keys and its lookup"],
  },
  {
    name: "option-declared-without-null",
    breaks:
      "an `Option` is declared `T | undefined`, so a TypeScript caller's `null`, which rust-js reads as `None`, doesn't typecheck",
    file: "src/lower/declarations/representation.rs",
    find: '        types.extend([keyword("null"), keyword("undefined")]);\n',
    replace: '        types.extend([keyword("undefined")]);\n',
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "boxed-option-declared-unboxed",
    breaks: "an `Option` whose `Some` lowering boxes, `Option<()>`, is declared its payload, `undefined | null | undefined`, not its box, `SomeNone | null | undefined` (#842)",
    file: "src/lower/declarations/representation.rs",
    find: "        if generic || top || !(self.representation.boxed)(self.owner, payload) {\n",
    replace: "        if true {\n",
    tests: ["test/declarations-boxed-options.test.ts"],
  },
  {
    name: "boxed-option-nullish-kept",
    breaks: "a boxed `Option<Option<u32>>`'s `Some` keeps its payload's `null | undefined`, `number | null | undefined | SomeNone`, which a `Some` never is (#842)",
    file: "src/lower/declarations/representation.rs",
    find: ".filter(|v| !nullish(v))",
    replace: ".filter(|_| true)",
    tests: ["test/declarations-boxed-options.test.ts"],
  },
  {
    name: "option-union-nested",
    breaks: "an `Option`'s union nests its `Some`'s, `(number | SomeNone) | null | undefined`, not one union (#842)",
    file: "src/lower/declarations/representation.rs",
    find: "            Some(\"union\") => some[\"types\"].as_array().cloned().unwrap_or_default(),\n",
    replace: "            Some(\"never\") => some[\"types\"].as_array().cloned().unwrap_or_default(),\n",
    tests: ["test/declarations-boxed-options.test.ts"],
  },
  {
    name: "mut-object-declared-box",
    breaks: "a `&mut` to an object is declared a box, `tick(counter: { value: Counter })`, where its JS takes the object (#619)",
    file: "src/lower/declarations/representation.rs",
    find: "        (self.representation.mut_ref)(self.owner, pointee)\n",
    replace: "        let _ = (self.representation.mut_ref)(self.owner, pointee);\n        MutRefForm::Box\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "box is declared"],
  },
  {
    name: "dyn-pair-declared-any",
    breaks: "a `Box<dyn Shape>` of a declared trait is declared `any`, not its pair `{ value; impl }` (#620)",
    file: "src/lower/declarations/representation.rs",
    find: "            Some(t) if self.declares_dictionary(t) => self.pair(t),\n",
    replace: "            Some(t) if self.declares_dictionary(t) => keyword(\"any\"),\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "box is declared"],
  },
  {
    name: "undeclared-dyn-pair-declared",
    breaks: "a `dyn` of a trait that isn't declared, `Box<dyn Steps>`, is declared a pair of `Steps<unknown>`, which no declaration declares (#620)",
    file: "src/lower/declarations/representation.rs",
    find: "            Some(t) if self.declares_dictionary(t) => self.pair(t),\n",
    replace: "            Some(t) if self.declares_dictionary(t) || t.is_local() => self.pair(t),\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "box is declared"],
  },
];
