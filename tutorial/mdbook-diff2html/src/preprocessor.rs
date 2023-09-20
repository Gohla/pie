use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mdbook::book::{Book, Chapter};
use mdbook::BookItem;
use mdbook::preprocess::PreprocessorContext;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};

#[derive(Default)]
pub struct Diff2Html {
  text: String,
  replacements: Vec<(Range<usize>, String)>,
}

impl Diff2Html {
  pub fn process_book(&mut self, context: &PreprocessorContext, book: &mut Book) -> Result<()> {
    let root_directory = &context.root;
    let source_directory = root_directory.join(&context.config.book.src);
    for item in &mut book.sections {
      self.process_item(item, &source_directory)?;
    }
    Ok(())
  }

  fn process_item(&mut self, item: &mut BookItem, source_directory: &Path) -> Result<()> {
    match item {
      BookItem::Chapter(chapter) => {
        self.process_chapter(chapter, source_directory)?;
        for sub_item in &mut chapter.sub_items {
          self.process_item(sub_item, source_directory)?;
        }
      }
      _ => {}
    };
    Ok(())
  }

  fn process_chapter(&mut self, chapter: &mut Chapter, source_directory: &Path) -> Result<()> {
    self.text.clear();
    self.replacements.clear();

    let mut in_diff = false;
    let mut from_file = false;
    let mut div_id_counter = 0;

    let parser = Parser::new(&chapter.content);
    for (event, range) in parser.into_offset_iter() {
      match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(s))) if s.contains("diff2html") => {
          in_diff = true;
          if s.contains("fromfile") {
            from_file = true;
          }
          self.text.clear();
        }
        Event::Text(t) if in_diff => {
          self.text.extend(t.chars());
        }
        Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(_))) if in_diff => {
          let html = if from_file {
            let file_path = Path::new(self.text.trim());
            let file_path = if file_path.is_relative() {
              to_absolute_path(source_directory, chapter.source_path.as_deref(), file_path)
                .with_context(|| format!("failed to create absolute path to diff file: {}", file_path.display()))?
            } else {
              file_path.to_path_buf()
            };
            let text = std::fs::read_to_string(&file_path)
              .with_context(|| format!("failed to read diff from: {}", file_path.display()))?;
            diff_to_html(&text, div_id_counter)
          } else {
            diff_to_html(&self.text, div_id_counter)
          };
          self.replacements.push((range, html));

          div_id_counter += 1;
          from_file = false;
          in_diff = false;
        }
        _ => {}
      }
    }

    // Note: applying replacements in reverse so that ranges are not invalidated.
    for (range, html) in self.replacements.drain(..).rev() {
      chapter.content.replace_range(range, &html)
    }

    Ok(())
  }
}

fn to_absolute_path(source_directory: &Path, source_file_path: Option<&Path>, relative_file_path: &Path) -> Result<PathBuf> {
  let source_file_path = source_file_path
    .context("no source file path available")?;
  let source_directory_path = source_file_path.parent()
    .with_context(|| format!("source file path '{}' has no parent", source_file_path.display()))?;
  Ok(source_directory.join(source_directory_path).join(relative_file_path))
}

fn diff_to_html(diff: &str, div_id_counter: usize) -> String {
  let diff = diff.replace('$', r#"${"$"}"#);
  let diff = diff.replace('`', r#"${"`"}"#);
  format!(r#"<div class="diff2html" id="diff2html_{div_id_counter}"></div>

<script>
  document.addEventListener('DOMContentLoaded', function () {{
    let diff = String.raw`{diff}`;
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
