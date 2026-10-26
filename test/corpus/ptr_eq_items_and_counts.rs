// React's hydration diff, as its port writes it: `ptr::eq` of a `Vec`'s
// item, an object, and a reference is whether the reference is the item,
// `node.children[0] !== skipToNode`, its index checked where nothing shows
// it in bounds, a panic past the end (ADR 0285, amended); a `*const` of
// what a shared reference refers to changes nothing, so an index the loop
// bounds after it is read as it is; a counter below a `String`'s length
// steps without a wrap; and a signed count of text's `repeat`, as it is,
// where text that isn't empty throws of a negative count as std panics of
// it (ADR 0774, amended), and empty text gives empty text.
use std::panic::{self, AssertUnwindSafe};

struct Node {
    name: &'static str,
    children: Vec<Node>,
}

fn leaf(name: &'static str) -> Node {
    Node {
        name,
        children: Vec::new(),
    }
}

fn indentation(indent: i32) -> String {
    "  ".to_owned() + &"  ".repeat(indent as usize)
}

fn find_notable_node(node: &Node) -> &Node {
    if node.children.len() == 1 {
        return find_notable_node(&node.children[0]);
    }
    node
}

fn describe_node(node: &Node, indent: i32) -> String {
    let skip_to_node = find_notable_node(node);
    if !std::ptr::eq(skip_to_node, node)
        && (node.children.len() != 1 || !std::ptr::eq(&node.children[0], skip_to_node))
    {
        return indentation(indent) + "...\n" + &describe_node(skip_to_node, indent + 1);
    }
    let mut content = indentation(indent) + node.name + "\n";
    let mut diff_idx = 0;
    let mut more = Some(2);
    while let Some(left) = more
        && diff_idx < node.children.len()
    {
        content += &describe_node(&node.children[diff_idx], indent + 1);
        diff_idx += 1;
        more = if left > 0 { Some(left - 1) } else { None };
    }
    content
}

fn is_first(items: &[Node], node: &Node) -> bool {
    std::ptr::eq(&items[0], node)
}

fn first_diff(server: &str, client: &str) -> u32 {
    let server_text = server.to_string();
    let mut client_text = client.to_string();
    let mut first_diff: u32 = 0;
    while first_diff < server_text.encode_utf16().count() as u32
        && first_diff < client_text.encode_utf16().count() as u32
    {
        if server_text.as_bytes().get(first_diff as usize) != client_text.as_bytes().get(first_diff as usize) {
            break;
        }
        first_diff += 1;
    }
    if first_diff > 3 {
        client_text = "...".to_owned() + &client_text;
    }
    first_diff + client_text.len() as u32
}

fn main() {
    let tree = Node {
        name: "root",
        children: vec![Node {
            name: "only",
            children: vec![Node {
                name: "deep",
                children: vec![leaf("a"), leaf("b"), leaf("c")],
            }],
        }],
    };
    print!("{}", describe_node(&tree, 0));
    print!("{}", describe_node(&tree.children[0].children[0], 1));
    let items = vec![leaf("x"), leaf("y")];
    println!("{} {}", is_first(&items, &items[0]), is_first(&items, &items[1]));
    let empty: Vec<Node> = Vec::new();
    match panic::catch_unwind(AssertUnwindSafe(|| is_first(&empty, &items[0]))) {
        Ok(first) => println!("no panic: {first}"),
        Err(payload) => println!("panicked: {:?}", payload.downcast_ref::<String>()),
    }
    println!("{} {}", first_diff("hello there", "hello world"), first_diff("ab", "ab"));
    let indent: i32 = -1;
    println!("[{}] [{}]", "".repeat(indent as usize), "ab".repeat(2_i32 as usize));
    println!("{:?}", panic::catch_unwind(|| "  ".repeat(indent as usize).len()).is_err());
}
