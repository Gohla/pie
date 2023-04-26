use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;

mod app;
mod modification;
mod output;
mod cargo;
mod stepper;
mod util;

#[derive(Parser, Debug)]
#[command()]
struct Args {
  #[arg(short, long)]
  info: bool,
  #[arg(short, long)]
  debug: bool,
}

fn main() {
  dotenv::dotenv().ok();

  let args = Args::parse();

  let mut level = None;
  if args.info {
    level = Some(LevelFilter::INFO.into());
  }
  if args.debug {
    level = Some(LevelFilter::DEBUG.into());
  }
  let filter = if let Some(level) = level {
    EnvFilter::builder()
      .with_default_directive(level)
      .parse_lossy("")
  } else {
    EnvFilter::from_env("MAIN_LOG")
  };

  let format = fmt::format()
    .with_level(false)
    .with_target(false)
    .with_thread_ids(false)
    .with_thread_names(false)
    .without_time()
    .compact();
  tracing_subscriber::registry()
    .with(
      fmt::layer()
        .event_format(format)
        .with_writer(std::io::stdout)
        .with_filter(filter)
    )
    .init();

  app::run();
}
