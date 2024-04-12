use std::path::PathBuf;

use clap::{command, Parser};
use env_logger;
use fundu::DurationParser;
use fundu::TimeUnit::{Day, Hour, Minute};
use log;
use std::io::{Error, ErrorKind, Result};
use std::time::{Duration, SystemTime};

mod cleaner;

/// Cleaner moves files matching specific conditions into trash or recycle bin
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to clean
    path: PathBuf,

    /// Don't delete, only print
    #[arg(short, long)]
    dry_run: bool,

    /// Duration to consider files for deletion, ex: 7d
    #[arg(short, long)]
    not_modified_within: Option<String>,
}

const PARSER: DurationParser = DurationParser::builder()
    .time_units(&[Minute, Hour, Day])
    .allow_negative()
    .build();

fn parse_time(time: String) -> Duration {
    let fundu_duration = PARSER.parse(time.as_str()).unwrap();
    Duration::try_from(fundu_duration).unwrap()
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    log::trace!("Starting Cleaner...");
    let args = Args::parse();

    let mut conditions: Vec<Box<dyn cleaner::CleanerCondition>> = Vec::new();
    let system_now = SystemTime::now();
    match args.not_modified_within {
        Some(value) => {
            let condition = Box::new(cleaner::LastModifiedIsOlderThan::new(
                parse_time(value),
                system_now,
            ));
            conditions.push(condition);
        }
        None => {}
    }
    if conditions.len() < 1 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "No conditions to check, please add some",
        ));
    }
    let cleaner = cleaner::Cleaner::new(args.path, args.dry_run, conditions);
    cleaner.clean()?;
    Ok(())
}
