// Mutations of src/lower/declarations.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/declarations.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "declared-option-required",
    breaks: "an `Option` field is a required prop to TypeScript, `text: string`, which a caller leaving it out is an error of",
    file: "src/lower/declarations.rs",
    find: '                    let _ = writeln!(out, "  {key}?: {};", self.ts(inner));',
    replace: '                    let _ = writeln!(out, "  {key}: {};", self.ts(inner));',
    tests,
  },
  {
    name: "declared-unit-enum-any",
    breaks: "a unit-only enum is `any` to TypeScript, which takes any string for it",
    file: "src/lower/declarations.rs",
    find: "        let union = match adt.variants().iter().all(|v| v.fields.is_empty()) {",
    replace: "        let union = match false {",
    tests,
  },
  {
    name: "declared-rest-dropped",
    breaks: "props a struct doesn't name, `...rest`, are an error to TypeScript, `aria-label` of a link",
    file: "src/lower/declarations.rs",
    find: '                let _ = writeln!(out, "  [prop: string]: unknown;");',
    replace: "",
    tests,
  },
  {
    name: "declared-binding-type-any",
    breaks: "a binding's type, react's `Memo<P>`, is `any` to TypeScript, which checks none of a `memo`'s props",
    file: "src/lower/declarations.rs",
    find: "                if let Some(declared) = tcx\n",
    replace: "                if false\n                    && let Some(declared) = tcx\n",
    tests,
  },
];
