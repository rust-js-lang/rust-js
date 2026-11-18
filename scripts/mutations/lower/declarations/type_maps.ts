// Mutations of src/lower/declarations/type_maps.rs, a trait whose impls for
// named unit structs are one TypeScript object type (ADR 1164).
import type { Mutation } from "../../../mutations";

const file = "src/lower/declarations/type_maps.rs";
const tests = ["test/declarations-type-maps.test.ts", "-t", "its keys and its lookup"];

export const mutations: Mutation[] = [
  {
    name: "type-map-associated-alias-erased",
    breaks: "an event alias that excludes null is normalized to its underlying Option",
    file: "src/lower/declarations/type_maps.rs",
    find: "        Some(self.as_written(written, |out| out.ts(ty)))",
    replace: "        Some(self.ts(ty))",
    tests: ["test/declarations-type-maps.test.ts", "-t", "written event aliases"],
  },
  {
    name: "type-map-lookup-of-others-never",
    breaks: "a type map's lookup of a string no impl names is `never`, where the trait's `&'static str` impl gives it `Event`, upstream's `EventOf<\"anything\">`",
    file,
    find: "                ty::Ref(_, str, _) if str.is_str() => otherwise = Some(value),\n",
    replace: "                ty::Ref(_, str, _) if str.is_str() => {}\n",
    tests,
  },
  {
    name: "type-map-keys-unwritten",
    breaks: "a type map's keys aren't declared, so `ChangeReason` is no type a caller can name",
    file,
    find: "                json!({ \"kind\": \"keyof\", \"type\": reference(&names.map) }),\n",
    replace: "                json!({ \"kind\": \"keyword\", \"keyword\": \"never\" }),\n",
    tests,
  },
  {
    name: "type-map-members-undocumented",
    breaks: "a type map's keys lose their structs' doc comments, which upstream's vocabulary carries for a caller's editor",
    file,
    find: "                    members.push(documented(member, tcx, adt.did()));\n",
    replace: "                    members.push(member);\n",
    tests,
  },
];
