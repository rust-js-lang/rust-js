// Mutations of src/lower/declarations/functions.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "parameter-declared-underscored",
    breaks: "a parameter's declaration keeps the `_` its JS name drops, `ref` declared `_ref`",
    file: "src/lower/declarations/functions.rs",
    find: "                        let named = super::super::parameter_name(self.tcx, def_id, ident.name.as_str());\n",
    replace: "                        let named = super::super::camel_case(ident.name.as_str());\n",
    tests: ["test/lowering-names.test.ts", "-t", "leading underscore"],
  },
  {
    name: "alias-param-unwritten",
    breaks: "a function's parameter written as an alias is its expansion, `(instance: HTMLElement | null) => void`, where it's `RefCallback<HTMLElement>`",
    file: "src/lower/declarations/functions.rs",
    find: "                let written = decl.and_then(|decl| decl.inputs.get(i));\n",
    replace: "                let written = None;\n",
    tests: ["test/declarations.test.ts", "-t", "aliases as the Rust names them"],
  },
  {
    name: "declared-param-rust-name",
    breaks: "a parameter named `new` is declared `new`, which TypeScript doesn't read, where the JS names it `new$`",
    file: "src/lower/declarations/functions.rs",
    find: "                        crate::names::js_ident(&named)\n",
    replace: "                        named\n",
    tests: ["test/declarations.test.ts", "-t", "as its JS does"],
  },
  {
    name: "trailing-option-param-required",
    breaks: "a Rust function's trailing `Option` parameter is declared required, `shown(name: string | null | undefined)`, so a TypeScript caller must give it",
    file: "src/lower/declarations/functions.rs",
    find: "                json!({ \"name\": name, \"optional\": i >= optional_from && !rest, \"rest\": rest, \"type\": ty })\n",
    replace: "                json!({ \"name\": name, \"optional\": false, \"rest\": rest, \"type\": ty })\n",
    tests: ["test/declarations.test.ts", "-t", "declarations take JS's null for an Option"],
  },
  {
    name: "written-return-expanded",
    breaks: "a function's result written as an alias is what it stands for, `TocItem[][]`, where it's `Toc[]`",
    file: "src/lower/declarations/functions.rs",
    find: "            (false, None) => self.as_written(written, |out| out.ts(output)),",
    replace: "            (false, None) => self.ts(output),",
    tests: ["test/declarations.test.ts", "-t", "declarations name a type alias"],
  },
  {
    name: "rest-declared-array",
    breaks: "a `.d.ts` declares a rest parameter an array argument, `xs: number[]`, which TypeScript callers give whole",
    file: "src/lower/declarations/functions.rs",
    find: "                let rest = rest == Some(i);\n",
    replace: "                let rest = rest == Some(i) && false;\n",
    tests: ["test/rest-parameters.test.ts", "-t", "rest slice is JS's rest parameter"],
  },
  {
    name: "nullable-result-declared-undefined",
    breaks: "a nullable function's result is declared `string | null | undefined`, though its `None` is `null`, never `undefined`",
    file: "src/lower/declarations/functions.rs",
    find: "            .then(|| self.option(output))\n",
    replace: "            .then(|| self.option(output))\n            .filter(|_| false)\n",
    tests: ["test/declarations.test.ts", "-t", "a nullable function's result is declared T or null"],
  },
  {
    name: "nullable-param-declared-optional",
    breaks: "a `#[rust_js::nullable(..)]` parameter is declared `fiber?: Fiber | null | undefined`, though it's taken as `Fiber | null` (ADR 0861, amended)",
    file: "src/lower/declarations/functions.rs",
    find: "                let ty = match nullable(i).then(|| self.option(ty)).flatten() {\n",
    replace: "                let ty = match None {\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "nullable thread-local's None"],
  },
];
