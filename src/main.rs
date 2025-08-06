use std::path::PathBuf;

use clap::{Parser, command};
use fundu::DurationParser;
use fundu::TimeUnit::{Day, Hour, Minute};
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

    /// Files to exclude from deletion (can be specified multiple times)
    #[arg(short, long)]
    exclude: Vec<String>,

    /// Disable default exclusions (.DS_Store, Thumbs.db, etc.)
    #[arg(long)]
    no_default_exclusions: bool,
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
    if let Some(value) = args.not_modified_within {
        let condition = Box::new(cleaner::LastModifiedIsOlderThan::new(
            parse_time(value),
            system_now,
        ));
        conditions.push(condition);
    }
    if conditions.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "No conditions to check, please add some",
        ));
    }

    // Build the exclusion filter
    let mut exclusion_patterns = Vec::new();

    // Add default exclusions unless disabled
    if !args.no_default_exclusions {
        exclusion_patterns.extend(cleaner::DEFAULT_EXCLUSIONS.iter().map(|s| s.to_string()));
    }

    // Add user-specified exclusions
    exclusion_patterns.extend(args.exclude);

    let exclusion_filter = cleaner::ExclusionFilter::new(exclusion_patterns);

    let cleaner = cleaner::Cleaner::new(args.path, args.dry_run, conditions, exclusion_filter);
    cleaner.clean()?;
    Ok(())
}
