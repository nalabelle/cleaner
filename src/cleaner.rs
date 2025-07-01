use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};
use std::time;

use log::{trace, warn};

pub trait CleanerCondition {
    fn test(&self, path: PathBuf) -> bool;
}

pub fn print_path(path: &Path) -> String {
    let formatted_path: &Path = match path.strip_prefix(".") {
        Ok(p) => p,
        Err(_) => path,
    };
    formatted_path.to_str().unwrap().to_string()
}

pub struct LastModifiedIsOlderThan {
    duration: time::Duration,
    now: time::SystemTime,
}

impl LastModifiedIsOlderThan {
    pub fn new(duration: time::Duration, now: time::SystemTime) -> LastModifiedIsOlderThan {
        trace!(
            "Adding LastModifiedIsOlderThan condition for {:?}",
            duration
        );
        LastModifiedIsOlderThan { duration, now }
    }

    fn modified_duration(&self, path: &PathBuf) -> time::Duration {
        let modified_time = fs::metadata(path).unwrap().modified().unwrap();
        self.now.duration_since(modified_time).unwrap()
    }
}

impl CleanerCondition for LastModifiedIsOlderThan {
    fn test(&self, path: PathBuf) -> bool {
        let modified = self.modified_duration(&path);
        trace!(
            "{}, File Time {:.0?} vs Comparison Time {:.0?}",
            print_path(&path),
            modified,
            self.duration
        );
        if modified > self.duration {
            return true;
        }
        false
    }
}

pub struct Cleaner {
    path: PathBuf,
    dry_run: bool,
    conditions: Vec<Box<dyn CleanerCondition>>,
}

impl Cleaner {
    pub fn new(
        path: PathBuf,
        dry_run: bool,
        conditions: Vec<Box<dyn CleanerCondition>>,
    ) -> Cleaner {
        Cleaner {
            path,
            dry_run,
            conditions,
        }
    }

    pub fn clean(&self) -> Result<()> {
        let entries = fs::read_dir(&self.path)?;
        let conditions = &self.conditions;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            for condition in conditions.iter() {
                let result = condition.test(path.clone());
                let postfix = if self.dry_run { " (dry run)" } else { "" };
                if result {
                    warn!("Deleting {}{}", print_path(&path), postfix);
                    if !self.dry_run {
                        trash::delete(&path).unwrap();
                    }
                }
            }
        }
        Ok(())
    }
}
