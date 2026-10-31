// rd15: a `match` of strings whose arms give `true` and `false` is a
// `switch`, as React DOM's isCustomElement writes `switch (tagName) { case
// 'annotation-xml': .. return false; default: return true; }`, not one
// boolean, `tagName !== "annotation-xml" && ..`. `matches!` is one
// boolean, as it's written.

fn is_custom_element(tag_name: &str) -> bool {
    if !tag_name.contains('-') {
        return false;
    }
    match tag_name {
        // These are reserved SVG and MathML elements.
        "annotation-xml" | "color-profile" | "missing-glyph" => false,
        _ => true,
    }
}

fn is_glyph(tag_name: &str) -> bool {
    match tag_name {
        "annotation-xml" | "color-profile" => false,
        "missing-glyph" => true,
        _ => false,
    }
}

fn is_reserved(tag_name: &str) -> bool {
    !matches!(tag_name, "annotation-xml" | "color-profile" | "missing-glyph")
}

fn main() {
    for tag in ["div", "my-element", "annotation-xml", "color-profile", "missing-glyph"] {
        println!("{tag}: {} {} {}", is_custom_element(tag), is_glyph(tag), is_reserved(tag));
    }
}
