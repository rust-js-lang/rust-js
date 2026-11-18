// Mutations of builtins/library.ts, the js crate's builtins written from
// TypeScript's own library (#1904).
import type { Mutation } from "../../mutations";

const file = "builtins/library.ts";
const test = (name: string) => ["test/intl-locale.test.ts", "-t", name];
// What's committed of what it writes, which a generator's change moves.
const written = ["test/builtins-coverage.test.ts", "-t", "Intl is what its template writes"];

export const mutations: Mutation[] = [
  {
    name: "library-inherited-members-lost",
    breaks: "a class's members are its own alone, so `Intl.Locale`'s `region` and `calendar`, its `LocaleOptions`', aren't bound",
    file,
    find: "  return [...own, ...inherited];",
    replace: "  return own;",
    tests: written,
  },
  {
    name: "library-optional-result-required",
    breaks: "an optional property's or an `undefined` result's Rust is its type alone, `region() -> String`, where JS gives `undefined`",
    file,
    find: "    results.set(member.name, optional ? `Option<${ty}>` : ty);",
    replace: "    results.set(member.name, ty);",
    tests: written,
  },
  {
    name: "library-throws-unread",
    breaks: "a constructor the policy says throws, `new Intl.Locale`, gives its object, not a `Result`, so a tag that isn't one panics nowhere and returns what JS threw",
    file,
    find: "    `new(${params.map((p: any) => p.rust).join(\", \")}) -> ${throws ? `Result<${made}, &'static JsError>` : made}`,",
    replace: "    `new(${params.map((p: any) => p.rust).join(\", \")}) -> ${made}`,",
    tests: written,
  },
  {
    name: "library-unbound-form-written-loose",
    breaks: "a type the generator doesn't map is written as any value, not refused by its name and its file",
    file,
    find: "  return refuse(`${at.owner}.${at.member}`, at.from, `a ${position === \"read\" ? \"result\" : \"given\"} type ${JSON.stringify(type)}`);",
    replace: "  return \"&'static JsValue\";",
    tests: test("the generator refuses a form it doesn't bind"),
  },

];
