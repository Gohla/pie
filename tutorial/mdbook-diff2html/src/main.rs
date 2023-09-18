use std::io;
use std::io::{BufReader, BufWriter};
use std::process;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use mdbook::preprocess::CmdPreprocessor;
use semver::{Version, VersionReq};

use preprocessor::Diff2Html;

mod preprocessor;

fn main() {
  let command = Command::new("mdbook-diff2html")
    .about("A mdbook preprocessor for showing highlighted diffs with diff2html")
    .subcommand(
      Command::new("supports")
        .arg(Arg::new("renderer").required(true))
        .about("Check whether a renderer is supported by this preprocessor"),
    );
  let matches = command.get_matches();

  let mut preprocessor = Diff2Html::default();
  if let Some(sub_args) = matches.subcommand_matches("supports") {
    handle_supports(sub_args);
  } else if let Err(e) = handle_preprocessing(&mut preprocessor) {
    eprintln!("{}", e);
    process::exit(1);
  }
}

fn handle_preprocessing(preprocessor: &mut Diff2Html) -> Result<()> {
  let (ctx, mut book) = CmdPreprocessor::parse_input(BufReader::new(io::stdin()))?;

  let book_version = Version::parse(&ctx.mdbook_version)?;
  let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;
  if !version_req.matches(&book_version) {
    eprintln!(
      "Warning: The mdbook-diff2html plugin was built against version {} of mdbook, but we're being called from version {}",
      mdbook::MDBOOK_VERSION,
      ctx.mdbook_version
    );
  }

  preprocessor.process_book(&ctx, &mut book)?;
  serde_json::to_writer(BufWriter::new(io::stdout()), &book)?;

  Ok(())
}

fn handle_supports(sub_args: &ArgMatches) -> ! {
  let renderer = sub_args
    .get_one::<String>("renderer")
    .expect("Required argument");
  if renderer == "html" { // Signal whether the renderer is supported by exiting with 1 or 0.
    process::exit(0);
  } else {
    process::exit(1);
  }
}
