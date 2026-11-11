use crate::*;
use glowkit_core::demo::{Category::Ai as C, Demo};

pub(crate) const SAMPLE_MD: &str = r#"## Ownership in Rust

Rust manages memory through **ownership**: every value has a single *owner*, and the value is dropped when its owner goes out of scope. You can read more in [the book](https://doc.rust-lang.org/book/).

1. Each value has an owner
2. There can only be **one owner** at a time
   - moves transfer ownership
   - `clone()` makes a deep copy
3. When the owner goes out of scope, the value is dropped

```rust
fn main() {
    let s = String::from("hello"); // s owns the string
    let t = s;                     // ownership moves to t
    println!("{t}, world!");
}
```

| Concept | Checked at | Cost |
|:--------|:----------:|-----:|
| Ownership | compile time | zero |
| Borrowing | compile time | zero |
| `Rc<T>` | run time | refcount |

> **Tip:** if the borrow checker complains, try restructuring data before reaching for `clone()`.

---

- [x] Read chapter 4
- [ ] Try the exercises
"#;

pub fn all() -> Vec<Demo> {
    vec![Demo {
        id: "ai-markdown",
        name: "Markdown",
        category: C,
        description: "Streaming-safe Markdown: headings, lists, tables, quotes, links and highlighted code blocks with a copy button.",
        covers: &["Streamdown: Markdown", "Vercel AI Elements: Response", "prompt-kit: Markdown", "assistant-ui: Markdown Text", "shadcn-chatbot-kit: Markdown Renderer"],
        code: r##"ui.add(Markdown::new(&response_text));"##,
        show: |ui| {
            ui.add(Markdown::new(SAMPLE_MD));
        },
    }]
}
