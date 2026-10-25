// Mutations of src/runtime/json_flat.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-flat-other-unsaid",
    breaks: "a number flattened is refused in other words than serde's",
    file: "src/runtime/json_flat.js",
    find: "    return $jsonError(\"can only flatten structs and maps\");\n",
    replace: "    return $jsonError(\"can only flatten structs\");\n",
    tests: ["test/serde.test.ts", "-t", "well formed or not"],
  },
  {
    name: "json-flat-struct-leaves-its-fields",
    breaks: "a flattened struct's fields are left for the fields after it, where serde takes them",
    file: "src/runtime/json_flat.js",
    find: "        this.entries[i] = null;\n        this.entry = entry;\n",
    replace: "        this.entry = entry;\n",
    tests: ["test/serde.test.ts", "-t", "well formed or not"],
  },
  {
    name: "json-flat-struct-takes-every-field",
    breaks: "a flattened struct takes every entry left, where serde takes those it has fields for",
    file: "src/runtime/json_flat.js",
    find: "      if (entry && entry[0].type === \"str\" && this.names.includes(entry[0].value)) {\n",
    replace: "      if (entry && entry[0].type === \"str\") {\n",
    tests: ["test/serde.test.ts", "-t", "well formed or not"],
  },
  {
    name: "json-flat-option-refuses",
    breaks: "a flattened `Option` of what can't be read is an error, where serde's is `None`",
    file: "src/runtime/json_flat.js",
    find: "      if (!(e instanceof $JsonError)) throw e;\n      return undefined;\n",
    replace: "      throw e;\n",
    tests: ["test/serde.test.ts", "-t", "well formed or not"],
  },
  {
    name: "json-flat-enum-unfound-unsaid",
    breaks: "a flattened enum with no variant there is refused in other words than serde's",
    file: "src/runtime/json_flat.js",
    find: "    if (i < 0) throw $jsonError(`no variant of enum ${enumName} found in flattened data`);\n",
    replace: "    if (i < 0) throw $jsonError(`no variant of enum ${enumName}`);\n",
    tests: ["test/serde.test.ts", "-t", "well formed or not"],
  },
  {
    name: "json-flat-enum-leaves-its-variant",
    breaks: "a flattened enum's variant is left for the map after it, where serde takes it",
    file: "src/runtime/json_flat.js",
    find: "    const entry = this.entries[i];\n    this.entries[i] = null;\n",
    replace: "    const entry = this.entries[i];\n",
    tests: ["test/serde.test.ts", "-t", "well formed or not"],
  },
  {
    name: "json-flat-content-of-taken",
    breaks: "what's left of a flattened struct for an internally tagged enum keeps the places of the entries taken",
    file: "src/runtime/json_flat.js",
    find: "    return { type: \"map\", value: this.entries.filter(Boolean) };\n",
    replace: "    return { type: \"map\", value: this.entries };\n",
    tests: ["test/serde.test.ts", "-t", "well formed or not"],
  },
];
