// Mutations of src/lower/classes/fields.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fields-declared-unasked",
    breaks: "an ES class that doesn't say `fields` declares them, react-art's `_args;` where upstream writes none, and one whose prototype has a setter of a field's name hides it from its constructor (#568)",
    file: "src/lower/classes/fields.rs",
    find: "        if super::home(tcx, adt) != Some(module) || !super::declares_fields(tcx, adt) {\n",
    replace: "        if super::home(tcx, adt) != Some(module) {\n",
    tests: ["test/es-classes.test.ts", "-t", "declares no fields unless it says so"],
  },
  {
    name: "fields-of-a-read-prototype-allowed",
    breaks: "`class(es, fields)` of a class whose prototype the program reads compiles, its fields declared over the setter it may define of a field's name (#568)",
    file: "src/lower/classes/fields.rs",
    find: "        if prototypes_read.contains(&adt) {\n",
    replace: "        if false {\n",
    tests: ["test/es-classes.test.ts", "-t", "fields that would change what it does are refused"],
  },
  {
    name: "fields-left-unset-allowed",
    breaks: "`class(es, fields)` of a class whose constructor leaves a field out, its `None`, compiles, and its fields are dropped, not declared or refused (#568)",
    file: "src/lower/classes/fields.rs",
    find: "    if let Some(unset) = keys.iter().find(|key| !set.contains(key)) {\n",
    replace: "    if let Some(unset) = keys.iter().find(|_| false) {\n",
    tests: ["test/es-classes.test.ts", "-t", "fields that would change what it does are refused"],
  },
  {
    name: "field-named-constructor-declared",
    breaks: "an ES class's field named `constructor` is declared, `constructor;`, a SyntaxError (#568)",
    file: "src/lower/classes/fields.rs",
    find: "        && !matches!(name, \"constructor\" | \"__proto__\")\n",
    replace: "        && !matches!(name, \"__proto__\")\n",
    tests: ["test/es-classes.test.ts", "-t", "fields that would change what it does are refused"],
  },
  {
    name: "fields-never-declared",
    breaks: "an ES class that says `fields` declares none of them, where upstream's, `chunkName;`, are (#568)",
    file: "src/lower/classes/fields.rs",
    find: "                    class.fields = fields.into_iter().map(|name| (at(&name), name)).collect();\n",
    replace: "                    class.fields = fields.into_iter().take(0).map(|name| (at(&name), name)).collect();\n",
    tests: ["test/es-classes.test.ts", "-t", "whose struct says es is an ES class"],
  },
];
