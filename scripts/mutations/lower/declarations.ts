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
  {
    name: "flattened-declared-nested",
    breaks: "a flattened field is declared a field, `anchor: Anchor`, not `extends Anchor`",
    file: "src/lower/declarations.rs",
    find: "            Some((ts, shadowed)) if shadowed.is_empty() => format!(\" extends {ts}\"),",
    replace: "            Some((ts, shadowed)) if false && shadowed.is_empty() => format!(\" extends {ts}\"),",
    tests,
  },
  {
    name: "flattened-extends-any",
    breaks: "another module's flattened struct is `extends any`, which TypeScript refuses",
    file: "src/lower/declarations.rs",
    find: "            Some((ts, _)) if ts == \"any\" => String::new(),",
    replace: "            Some((ts, _)) if false && ts == \"any\" => String::new(),",
    tests,
  },
  {
    name: "shadowed-names-not-omitted",
    breaks: "props extend what they shadow, not `Omit<Linked, \"href\" | \"className\">`",
    file: "src/lower/declarations.rs",
    find: "            Some((ts, shadowed)) => format!(\" extends Omit<{ts}, {}>\", shadowed.join(\" | \")),",
    replace: "            Some((ts, shadowed)) => format!(\" extends {ts}{}\", shadowed.len() * 0),",
    tests: tests,
  },
  {
    name: "generic-type-imported-whole",
    breaks: "`AnchorHTMLAttributes<HTMLAnchorElement>` is imported by that whole text",
    file: "src/lower/declarations.rs",
    find: "                            let imported = name.split('<').next().unwrap_or(name);",
    replace: "                            let imported = name;",
    tests: tests,
  },
  {
    name: "chain-keys-not-followed",
    breaks: "a name shadowed down the chain, `className`, isn't omitted",
    file: "src/lower/declarations.rs",
    find: "            keys.extend(flattened_keys(tcx, field.ty(tcx, args).skip_normalization()));",
    replace: "            keys.push(field_key(tcx, field));",
    tests: tests,
  },
];
