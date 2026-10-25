// Mutations of src/lower/bindings/rest_values.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rest-value-unrecognized",
    breaks: "a `&dyn js::Rest<T>` isn't rest arguments: `console::log` taken as a `fn` of one is refused",
    file: "src/lower/bindings/rest_values.rs",
    find: "                .is_some_and(|t| tcx.get_attrs_by_path(t, &path).next().is_some()),\n",
    replace: "                .is_some_and(|t| tcx.get_attrs_by_path(t, &path).next().is_some() && false),\n",
    tests: ["test/js-object.test.ts", "-t", "console is a value"],
  },
  {
    name: "rest-items-undeclared",
    breaks: "a function type's rest arguments as a value are declared one parameter of `any`, not `...args: string[]`",
    file: "src/lower/bindings/rest_values.rs",
    find: "    (!matches!(read.kind(), ty::Alias(..))).then_some(read)\n",
    replace: "    (!matches!(read.kind(), ty::Alias(..))).then_some(read).filter(|_| false)\n",
    tests: ["test/rest-parameters.test.ts", "-t", "spreads them, through any type"],
  },
];
