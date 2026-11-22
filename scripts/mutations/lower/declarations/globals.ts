// Mutations of crates/rust-js-lower/src/declarations/globals.rs (ADR 0093): a type
// TypeScript's DOM lacks, which a binding says rust-js declares,
// `declare global { .. }` of its binding (#1790).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "declares-never-declared",
    breaks: "a crate's declarations that name `BatteryManager`, which TypeScript's DOM lacks, don't declare it, so a consumer's strict tsc finds no such type",
    file: "crates/rust-js-lower/src/declarations/globals.rs",
    find: "        self.unexported\n            .insert(key, Some(json!({ \"kind\": \"global\", \"declarations\": [interface] })));\n",
    replace: "        let _ = (interface, key);\n",
    tests: ["test/webapi-declares.test.ts"],
  },
  {
    name: "declares-every-accessor-readonly",
    breaks: "a property its binding sets, `onlevelchange`, is declared `readonly`, so a consumer can't set the handler it may",
    file: "crates/rust-js-lower/src/declarations/globals.rs",
    find: "                        \"readonly\": !set.contains(&property),\n",
    replace: "                        \"readonly\": true,\n",
    tests: ["test/webapi-declares.test.ts", "-t", "getBattery"],
  },
  {
    name: "declares-extends-nothing",
    breaks: "a declared type is no class's it derefs to, `interface BatteryManager {` not `extends EventTarget`",
    file: "crates/rust-js-lower/src/declarations/globals.rs",
    find: "            .into_iter()\n            .collect();\n        json!({",
    replace: "            .into_iter()\n            .filter(|_| false)\n            .collect();\n        json!({",
    tests: ["test/webapi-declares.test.ts", "-t", "getBattery"],
  },
];
