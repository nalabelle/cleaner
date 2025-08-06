use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};
use std::time;

use log::{trace, warn};

#[cfg(test)]
use std::time::{Duration as StdDuration, SystemTime};

pub trait CleanerCondition {
    fn test(&self, path: PathBuf) -> bool;
}

pub fn print_path(path: &Path) -> String {
    let formatted_path: &Path = match path.strip_prefix(".") {
        Ok(p) => p,
        Err(_) => path,
    };
    formatted_path
        .to_str()
        .unwrap_or("<non-utf8 path>")
        .to_string()
}

pub struct LastModifiedIsOlderThan {
    duration: time::Duration,
    now: time::SystemTime,
}

impl LastModifiedIsOlderThan {
    pub fn new(duration: time::Duration, now: time::SystemTime) -> LastModifiedIsOlderThan {
        trace!("Adding LastModifiedIsOlderThan condition for {duration:?}");
        LastModifiedIsOlderThan { duration, now }
    }

    fn modified_duration(&self, path: &PathBuf) -> time::Duration {
        let meta = fs::metadata(path).unwrap_or_else(|e| {
            panic!(
                "Failed to read metadata for '{}': {}",
                print_path(path.as_path()),
                e
            )
        });
        let modified_time = meta.modified().unwrap_or_else(|e| {
            panic!(
                "Failed to read modified time for '{}': {}",
                print_path(path.as_path()),
                e
            )
        });
        self.now.duration_since(modified_time).unwrap_or_else(|e| {
            panic!(
                "Failed to compute duration since last modified for '{}': {}. now={:?}, modified_time={:?}. This often indicates the file's modified time is in the future relative to system clock.",
                print_path(path.as_path()),
                e,
                self.now,
                modified_time
            )
        })
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
        let mut trash_ctx = trash::TrashContext::new();

        #[cfg(target_os = "macos")]
        // Use NSFileManager method which doesn't produce sound on macOS
        trash_ctx.set_delete_method(DeleteMethod::NsFileManager);

        Cleaner {
            path,
            dry_run,
            conditions,
            trash_ctx,
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
                        self.trash_ctx.delete(&path).unwrap_or_else(|e| {
                            panic!("Failed to move to trash '{}': {}", print_path(&path), e)
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    // Cross-platform helper: set mtime using the filetime crate
    fn set_mtime(path: &Path, mtime: std::time::SystemTime) {
        use filetime::{FileTime, set_file_mtime};
        let ft_m = FileTime::from_system_time(mtime);
        set_file_mtime(path, ft_m).unwrap();
    }

    #[test]
    fn print_path_handles_non_utf8() {
        // On most systems it's hard to create truly non-utf8 paths portably; this at least ensures no panic on normal paths
        let p = PathBuf::from("./some/relative/path.txt");
        let s = print_path(&p);
        assert!(s.ends_with("some/relative/path.txt"));
    }

    #[test]
    fn condition_deletes_when_older_than() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("old.txt");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, "test").unwrap();
        }

        // Make file older by 2 hours
        let now = SystemTime::now();
        let two_hours = StdDuration::from_secs(2 * 60 * 60);
        let file_mtime = now - two_hours;
        set_mtime(&file_path, file_mtime);

        let cond = LastModifiedIsOlderThan::new(two_hours - StdDuration::from_secs(1), now);
        assert!(cond.test(file_path.clone()));
    }

    #[test]
    fn condition_skips_when_newer_than() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("new.txt");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, "test").unwrap();
        }

        // Make file just 10 seconds old
        let now = SystemTime::now();
        let ten_secs = StdDuration::from_secs(10);
        let file_mtime = now - ten_secs;
        set_mtime(&file_path, file_mtime);

        let cond = LastModifiedIsOlderThan::new(StdDuration::from_secs(60), now);
        assert!(!cond.test(file_path.clone()));
    }

    #[test]
    #[should_panic(expected = "Failed to compute duration since last modified")]
    fn condition_panics_with_clear_message_on_future_mtime() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("future.txt");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, "test").unwrap();
        }

        // Make file 5 minutes in the future to trigger SystemTimeError
        let now = SystemTime::now();
        let future = now + StdDuration::from_secs(5 * 60);
        set_mtime(&file_path, future);

        let cond = LastModifiedIsOlderThan::new(StdDuration::from_secs(60), now);
        // this will execute modified_duration internally and should panic with our improved message
        let _ = cond.test(file_path.clone());
    }

    #[test]
    fn cleaner_respects_dry_run_and_condition() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("candidate.txt");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, "test").unwrap();
        }

        // Make file 2 days old
        let now = SystemTime::now();
        let two_days = StdDuration::from_secs(2 * 24 * 60 * 60);
        let file_mtime = now - two_days;
        set_mtime(&file_path, file_mtime);

        let cond = Box::new(LastModifiedIsOlderThan::new(
            StdDuration::from_secs(24 * 60 * 60),
            now,
        ));
        let cleaner = Cleaner::new(dir.path().to_path_buf(), true, vec![cond]); // dry-run

        // Should not panic and not actually delete the file in dry-run
        cleaner.clean().unwrap();
        assert!(file_path.exists());
    }
}
