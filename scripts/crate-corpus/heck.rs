use heck::{
    AsKebabCase, ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase,
    ToTrainCase, ToUpperCamelCase,
};

pub fn report() -> String {
    let mut lines = Vec::new();
    for text in [
        "XMLHttpRequest",
        "hello world",
        "CamelCase",
        "ΣΑΣ ΟΔΟΣ",
        "ßtraße two",
        "",
        "  a__b  ",
    ] {
        lines.push(format!(
            "{text:?}: {} {} {} {} {} {} {} {}",
            text.to_snake_case(),
            text.to_kebab_case(),
            text.to_lower_camel_case(),
            text.to_upper_camel_case(),
            text.to_shouty_snake_case(),
            text.to_shouty_kebab_case(),
            text.to_title_case(),
            text.to_train_case(),
        ));
    }
    lines.push(format!("{:>12}|", AsKebabCase("Padded Here")));
    lines.join("\n")
}
