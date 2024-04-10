use log;
use std::fs;
use std::io::Result;
use std::path::PathBuf;
use std::time;
use trash;

pub trait CleanerCondition {
    fn test(&self, path: PathBuf) -> bool;
    fn display(&self) -> String;
}

pub fn print_path(path: &PathBuf) -> String {
    format!("{}", path.strip_prefix(".").unwrap().to_str().unwrap())
}

pub struct LastModifiedIsOlderThan {
    duration: time::Duration,
    now: time::SystemTime,
}

impl LastModifiedIsOlderThan {
    pub fn new(duration: time::Duration, now: time::SystemTime) -> LastModifiedIsOlderThan {
        log::trace!(
            "Adding LastModifiedIsOlderThan condition for {:?}",
            duration
        );
        LastModifiedIsOlderThan {
            duration: duration,
            now: now,
        }
    }

    fn modified_duration(&self, path: &PathBuf) -> time::Duration {
        let modified_time = fs::metadata(path).unwrap().modified().unwrap();
        self.now.duration_since(modified_time).unwrap()
    }
}

impl CleanerCondition for LastModifiedIsOlderThan {
    fn display(&self) -> String {
        format!("LastModifiedIsOlderThan {:?}", &self.duration)
    }

    fn test(&self, path: PathBuf) -> bool {
        let modified = self.modified_duration(&path);
        if modified > self.duration {
            log::debug!(
                "Would delete /{}, File Time {:.0?} vs Comparison Time {:.0?}",
                print_path(&path),
                modified,
                self.duration
            );
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
            path: path,
            dry_run: dry_run,
            conditions: conditions,
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
                if result {
                    log::info!("Deleting {:?}", print_path(&path));
                    if !self.dry_run {
                        trash::delete(&path).unwrap();
                    }
                }
            }
        }
        Ok(())
    }
}
