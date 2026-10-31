// Mutations of src/lower/bindings/imports.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "namespace-name-unread",
    breaks:
      "`scheduler#* as Scheduler.unstable_now` is the export `* as Scheduler`, not the module itself named `Scheduler`",
    file: "src/lower/bindings/imports.rs",
    find: '    if let Some(named) = path.strip_prefix("* as ") {\n',
    replace:
      '    if let Some(named) = path.strip_prefix("* as ").filter(|_| false) {\n',
    tests: [
      "test/modules.test.ts",
      "-t",
      "a namespace import is named as its link_name",
    ],
  },
];
