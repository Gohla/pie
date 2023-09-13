use anyhow::Result;
use mdbook::book::{Book, Chapter};
use mdbook::BookItem;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};

pub struct Diff2Html;

impl Diff2Html {
  pub fn new() -> Self { Self }
}

impl Preprocessor for Diff2Html {
  fn name(&self) -> &str { "diff2html" }
  fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
    book.for_each_mut(|item| {
      match item {
        BookItem::Chapter(chapter) => self.process_chapter(chapter),
        _ => {},
      };
    });
    Ok(book)
  }
  fn supports_renderer(&self, renderer: &str) -> bool { renderer == "html" }
}

impl Diff2Html {
  fn process_chapter(&self, chapter: &mut Chapter) {
    let mut in_diff = false;
    let mut text = String::new();
    let mut div_id_counter = 0;
    let mut replacements = Vec::new();

    let parser = Parser::new(&chapter.content);
    for (event, range) in parser.into_offset_iter() {
      match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(s))) if s.contains("diff2html") => {
          in_diff = true;
          text.clear();
        },
        Event::Text(t) if in_diff => {
          text.extend(t.chars());
        },
        Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(_))) if in_diff => {
          let html = self.create_diff(&text, div_id_counter);
          replacements.push((range, html));
          div_id_counter += 1;
          in_diff = false;
        },
        _ => {}
      }
    }

    for (range, html) in replacements {
      chapter.content.replace_range(range, &html)
    }
  }

  fn create_diff(&self, diff: &str, div_id_counter: usize) -> String {
    format!(r#"<div id="diff2html_{div_id_counter}"></div>

<script>
  document.addEventListener('DOMContentLoaded', function () {{
    let diff = `{diff}`;
    let target = document.getElementById('diff2html_{div_id_counter}');
    let configuration = {{
      drawFileList: false,
      fileListToggle: false,
      fileContentToggle: false,
      
      outputFormat: 'side-by-side',
      matching: 'lines',
    }};
    let diff2htmlUi = new Diff2HtmlUI(target, diff, configuration, hljs);
    diff2htmlUi.draw();
  }});
</script>
"#)
  }
}
