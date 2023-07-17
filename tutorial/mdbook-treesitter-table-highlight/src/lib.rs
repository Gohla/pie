use std::fmt::Write;

use anyhow::Result;
use mdbook::book::Book;
use mdbook::BookItem;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Parser, Tag};
use pulldown_cmark_to_cmark::cmark;
use tree_sitter_highlight::{HighlightConfiguration, Highlighter, HtmlRenderer};

pub struct TreesitterTableHighlight {
  rust_highlight_config: HighlightConfiguration,
  _toml_highlight_config: HighlightConfiguration,
}

impl TreesitterTableHighlight {
  pub fn new() -> Result<Self> {
    let mut rust_highlight_config = HighlightConfiguration::new(
      tree_sitter_rust::language(),
      tree_sitter_rust::HIGHLIGHT_QUERY,
      "",
      "",
    )?;
    let mut toml_highlight_config = HighlightConfiguration::new(
      tree_sitter_toml::language(),
      tree_sitter_toml::HIGHLIGHT_QUERY,
      "",
      ""
    )?;

    rust_highlight_config.configure(&HIGHLIGHT_NAMES);
    toml_highlight_config.configure(&HIGHLIGHT_NAMES);

    Ok(Self {
      rust_highlight_config,
      _toml_highlight_config: toml_highlight_config,
    })
  }

  fn highlight_rust_source_code(&self, source: &[u8]) -> String {
    let mut highlighter = Highlighter::new();
    let highlights = highlighter.highlight(&self.rust_highlight_config, source, None, |_| None).unwrap();

    let mut renderer = HtmlRenderer::new();
    renderer.render(highlights, source, &|h| ATTRIBUTES[h.0].as_bytes()).unwrap();

    let mut html = String::new();
    writeln!(html, "<pre>").unwrap();
    for line in renderer.lines() {
      writeln!(html, "{}", line).unwrap();
    }
    writeln!(html, "</pre>").unwrap();
    html
  }
}

impl Preprocessor for TreesitterTableHighlight {
  fn name(&self) -> &str { "treesitter-table-highlight" }
  fn supports_renderer(&self, renderer: &str) -> bool { renderer == "html" }
  fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
    book.for_each_mut(|item| {
      match item {
        BookItem::Chapter(chapter) => {
          let mut in_source_code = false;
          let mut source = String::new();
          let mut source_language = "";
          let events = Parser::new(&chapter.content).filter_map(|e| match &e {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(s))) => {
              if s.contains("rust") {
                eprintln!("Start Rust source code");
                in_source_code = true;
                source.clear();
                source_language = "rust";
                None
              } else {
                Some(e)
              }
            },
            Event::Text(t) => {
              if in_source_code {
                eprintln!("Append source code");
                source.extend(t.chars());
                None
              } else {
                Some(e)
              }
            },
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
              if in_source_code {
                if source_language == "rust" {
                  eprintln!("End Rust source code");
                  let html = self.highlight_rust_source_code(source.as_bytes());
                  in_source_code = false;
                  source.clear();
                  source_language = "";
                  eprintln!("{}", html);
                  Some(Event::Html(CowStr::from(html)))
                } else {
                  Some(e)
                }
              } else {
                Some(e)
              }
            },
            _ => Some(e),
          });

          let mut buf = String::with_capacity(chapter.content.len());
          cmark(events, &mut buf).unwrap();
          chapter.content = buf;
        },
        _ => {},
      };
    });

    Ok(book)
  }
}

const HIGHLIGHT_NAMES: [&'static str; 27] = [
  "type",
  "constructor",
  "constant",
  "constant.builtin",
  "constant.character",
  "constant.character.escape",
  "string",
  "string.regexp",
  "string.special",
  "comment",
  "variable",
  "variable.parameter",
  "variable.builtin",
  "variable.other.member",
  "label",
  "punctuation",
  "punctuation.special",
  "keyword",
  "keyword.storage.modifier.ref",
  "keyword.control.conditional",
  "operator",
  "function",
  "function.macro",
  "tag",
  "attribute",
  "namespace",
  "special",
  // "markup.heading.marker",
  // "markup.heading.1",
  // "markup.heading.2",
  // "markup.heading.3",
  // "markup.heading.4",
  // "markup.heading.5",
  // "markup.heading.6",
  // "markup.list",
  // "markup.bold",
  // "markup.italic",
  // "markup.strikethrough",
  // "markup.link.url",
  // "markup.link.text",
  // "markup.raw",
  // "diff.plus",
  // "diff.minus",
  // "diff.delta",
];

const ATTRIBUTES: [&'static str; 27] = [
  "class = \"hljs-type\"",
  "class = \"hljs-title function_\"",
  "class = \"hljs-variable constant_\"",
  "class = \"hljs-built_in\"",
  "class = \"hljs-symbol\"",
  "class = \"hljs-symbol\"",
  "class = \"hljs-string\"",
  "class = \"hljs-regexp\"",
  "class = \"hljs-string\"",
  "class = \"hljs-comment\"",
  "class = \"hljs-variable\"",
  "class = \"hljs-params\"",
  "class = \"hljs-built_in\"",
  "class = \"hljs-variable\"",
  "class = \"hljs-symbol\"",
  "class = \"hljs-punctuation\"",
  "class = \"hljs-punctuation\"",
  "class = \"hljs-keyword\"",
  "class = \"hljs-keyword\"",
  "class = \"hljs-keyword\"",
  "class = \"hljs-operator\"",
  "class = \"hljs-title function_\"",
  "class = \"hljs-title function_\"",
  "class = \"hljs-tag\"",
  "class = \"hljs-attribute\"",
  "class = \"hljs-title class_\"",
  "class = \"hljs-literal\"",
];
