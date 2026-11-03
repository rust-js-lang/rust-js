// A `let` the program wrote, named as the field it reads, whose one read
// is the left side of the `||` or `??` the statement right after it
// evaluates first, is the field read there: React's readAsDataURL's
// `const mimeType = blob.type || 'application/octet-stream';` of `let
// r#type = blob.r#type(); let mime_type = if r#type.is_empty() { .. } else
// { &r#type };` (rd664, the user's decision, an exception to rd226). Read
// twice, after something else, named otherwise, or past the statement
// after it, the `let` stays, as written.
struct Blob {
    r#type: String,
    name: Option<String>,
}

fn mime(blob: &Blob) -> String {
    let r#type = blob.r#type.clone();
    let mime_type = if r#type.is_empty() {
        "application/octet-stream"
    } else {
        &r#type
    };
    "data:".to_owned() + mime_type
}

fn named(blob: &Blob) -> String {
    let name = blob.name.clone();
    name.unwrap_or_else(|| "unknown".to_owned())
}

fn twice(blob: &Blob) -> String {
    let r#type = blob.r#type.clone();
    let mime_type = if r#type.is_empty() { "none" } else { &r#type };
    mime_type.to_owned() + "/" + &r#type
}

fn not_first(blob: &Blob, prefix: &str) -> String {
    let r#type = blob.r#type.clone();
    prefix.to_owned() + if r#type.is_empty() { "none" } else { &r#type }
}

fn other_name(blob: &Blob) -> String {
    let kind = blob.r#type.clone();
    let mime_type = if kind.is_empty() { "none" } else { &kind };
    mime_type.to_owned()
}

fn not_next(blob: &mut Blob) -> String {
    let r#type = blob.r#type.clone();
    blob.r#type = "changed".to_owned();
    let mime_type = if r#type.is_empty() { "none" } else { &r#type };
    mime_type.to_owned()
}

fn main() {
    let mut blobs = [
        Blob {
            r#type: String::new(),
            name: None,
        },
        Blob {
            r#type: "text/plain".to_owned(),
            name: Some("a.txt".to_owned()),
        },
    ];
    for blob in &mut blobs {
        println!("{} {}", mime(blob), named(blob));
        println!("{} {}", twice(blob), not_first(blob, "type: "));
        println!("{} {} {}", other_name(blob), not_next(blob), blob.r#type);
    }
}
