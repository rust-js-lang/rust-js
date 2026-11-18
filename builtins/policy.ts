// What TypeScript's library doesn't say of a builtin that its Rust binding
// needs, each entry the fact it lacks, its owner and the test that proves
// it (#1904). Never a member or a signature: those are the library's, which
// `builtins/library.ts` reads. An entry may name, or say what throws; it
// may not narrow a declared union, an optional key or an overload.

/** A Rust name for what the library leaves unnamed, or names otherwise. */
export type Named = { rust: string; why: string; proof: string };

/** Each by its library name: an alias, an inline union `Owner.member`'s,
 * or a parameter's union, `new Owner(param)`'s. */
export const NAMES: Record<string, Named> = {
  "Intl.LocaleCollationCaseFirst": {
    rust: "CaseFirst",
    why: "the established Rust name of `caseFirst`'s values, which the collator's options share; its members are the alias's",
    proof: "test/intl-locale.test.ts -t \"a locale's options and its parts\"",
  },
  "Intl.LocaleHourCycleKey": {
    rust: "HourCycle",
    why: "the established Rust name of `hourCycle`'s values, which the date formatter's options share; its members are the alias's",
    proof: "test/intl-locale.test.ts -t \"a locale's options and its parts\"",
  },
  "Intl.TextInfo.direction": {
    rust: "TextDirection",
    why: "an inline union of `\"ltr\" | \"rtl\"`, which TypeScript names nothing",
    proof: "test/intl-locale.test.ts -t \"what a locale knows of its text, its week and its time zones\"",
  },
  "new Intl.Locale(tag)": {
    rust: "IntoLocaleTag",
    why: "the sealed input of `string | Intl.Locale`, which TypeScript names nothing; not `IntoLocalesArgument`, whose lists a tag isn't",
    proof: "test/intl-locale.test.ts -t \"a locale of a tag or of another locale\"",
  },
};

/** What throws, which a TypeScript signature doesn't say: its error. */
export type Throws = { error: "RangeError" | "TypeError"; why: string; proof: string };

/** Each by its call's JS, `new Intl.Locale`. */
export const THROWS: Record<string, Throws> = {
  "new Intl.Locale": {
    error: "RangeError",
    why: "ECMA-402's `Intl.Locale` throws a `RangeError` of a tag that isn't a well-formed language tag",
    proof: "test/intl-locale.test.ts -t \"a locale of a tag or of another locale\"",
  },
};
