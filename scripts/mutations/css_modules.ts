// Mutations of src/css_modules.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "css-module-icss-unrefused",
    breaks:
      "a CSS Module of ICSS's `:export { primary: red }` gives its keys without `primary`, which css-loader exports, so a binding of it is refused or taken by the bundler it's built with (#1617, ADR 1133)",
    file: "src/css_modules.rs",
    find: "    if let Some(name) = sheet.rules.0.iter().find_map(icss_rule) {\n",
    replace: "    if let Some(name) = sheet.rules.0.iter().find_map(icss_rule).filter(|_| false) {\n",
    tests: ["test/css-modules.test.ts", "-t", "ICSS"],
  },
  {
    name: "css-module-error-line-from-zero",
    breaks: "a CSS Module Lightning CSS can't read is said to be wrong a line before where it is, its line counted from 0 (ADR 1133)",
    file: "src/css_modules.rs",
    find: '        format!(", at line {}, column {}", at.line + 1, at.column)\n',
    replace: '        format!(", at line {}, column {}", at.line, at.column)\n',
    tests: ["test/css-modules.test.ts", "-t", "ICSS"],
  },
];
