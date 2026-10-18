use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::error::{AppError, AppResult};
use super::ignore_matcher::IgnoreMatcher;
use super::models::{FolderEntry, ScanOptions, ScanReport, ScanStats};
use super::path_guard;
use super::timestamp;

const RESULT_BATCH_SIZE: usize = 128;
const CHANNEL_CAPACITY: usize = 512;
const PROGRESS_INTERVAL: i64 = 256;
const MAX_DIRECTORY_DEPTH: usize = 512;

struct Counters {
    visited: AtomicI64,
    empty: AtomicI64,
}

impl Counters {
    fn new() -> Self {
        Self {
            visited: AtomicI64::new(0),
            empty: AtomicI64::new(0),
        }
    }

    fn increment_visited(&self) {
        self.visited.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_empty(&self) {
        self.empty.fetch_add(1, Ordering::SeqCst);
    }

    fn to_stats(&self, elapsed: Duration) -> ScanStats {
        ScanStats {
            visited_directories: self.visited.load(Ordering::SeqCst),
            empty_folders: self.empty.load(Ordering::SeqCst),
            elapsed: timestamp::format_duration(elapsed),
        }
    }
}

struct ProducerState {
    sender: SyncSender<FolderEntry>,
    counters: Arc<Counters>,
    cancel: Arc<AtomicBool>,
    matcher: Arc<IgnoreMatcher>,
}

pub fn scan<B, P>(
    options: &ScanOptions,
    cancel: Arc<AtomicBool>,
    mut on_batch: B,
    mut on_progress: P,
) -> AppResult<ScanReport>
where
    B: FnMut(&[FolderEntry]),
    P: FnMut(&ScanStats),
{
    let Some(root) = path_guard::try_normalize(Some(&options.root_directory)) else {
        return Err(AppError::validation("root_directory_invalid"));
    };
    if !Path::new(&root).is_dir() {
        return Err(AppError::general("not_found"));
    }

    let matcher = Arc::new(IgnoreMatcher::new(
        &options.ignored_paths,
        &options.ignore_patterns,
    ));
    let workers = options.max_degree_of_parallelism.clamp(
        1,
        std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1)
            .max(1),
    );

    let roots = enumerate_scan_roots(&root, &matcher)?;
    let counters = Arc::new(Counters::new());
    let (sender, receiver) = sync_channel::<FolderEntry>(CHANNEL_CAPACITY);
    let start = Instant::now();
    let next_root = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..workers.min(roots.len().max(1)) {
        let state = ProducerState {
            sender: sender.clone(),
            counters: counters.clone(),
            cancel: cancel.clone(),
            matcher: matcher.clone(),
        };
        let roots = roots.clone();
        let next_root = next_root.clone();
        handles.push(std::thread::spawn(move || loop {
            if state.cancel.load(Ordering::SeqCst) {
                break;
            }
            let index = next_root.fetch_add(1, Ordering::SeqCst);
            let Some(directory) = roots.get(index) else {
                break;
            };
            scan_subtree(directory, &state);
        }));
    }
    drop(sender);

    let mut batch: Vec<FolderEntry> = Vec::with_capacity(RESULT_BATCH_SIZE);
    let mut was_cancelled = false;
    let mut last_visited = 0i64;
    let mut failure_message = None;

    while let Ok(entry) = receiver.recv() {
        batch.push(entry);
        if batch.len() >= RESULT_BATCH_SIZE {
            on_batch(&batch);
            batch.clear();
        }

        let visited = counters.visited.load(Ordering::SeqCst);
        if visited != last_visited && visited % PROGRESS_INTERVAL == 0 {
            last_visited = visited;
            on_progress(&counters.to_stats(start.elapsed()));
        }
    }

    for handle in handles {
        if handle.join().is_err() {
            failure_message = Some("scan_worker_failed".to_string());
        }
    }

    if !batch.is_empty() {
        on_batch(&batch);
    }

    if cancel.load(Ordering::SeqCst) {
        was_cancelled = true;
    }

    let stats = counters.to_stats(start.elapsed());
    on_progress(&stats);

    if let Some(message) = failure_message {
        return Err(AppError::general(message));
    }

    Ok(ScanReport {
        stats,
        was_cancelled,
    })
}

fn enumerate_scan_roots(root: &str, matcher: &IgnoreMatcher) -> AppResult<Vec<String>> {
    let mut roots: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(root)
        .map_err(|error| AppError::general(super::error_categories::from_io(&error)))?;
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if is_link(&path) || matcher.is_ignored(&path.to_string_lossy()) {
            continue;
        }
        roots.push(path.to_string_lossy().to_string());
    }
    Ok(roots)
}

fn scan_subtree(start_directory: &str, state: &ProducerState) {
    let mut pending: Vec<String> = vec![start_directory.to_string()];

    while let Some(current) = pending.pop() {
        if state.cancel.load(Ordering::SeqCst) {
            return;
        }

        if path_guard::get_depth(&current) > MAX_DIRECTORY_DEPTH {
            continue;
        }

        let path = PathBuf::from(&current);
        if !path.is_dir() || is_link(&path) {
            continue;
        }

        let mut has_children = false;
        let mut deferred: Vec<String> = Vec::new();
        match std::fs::read_dir(&path) {
            Ok(entries) => {
                for entry in entries {
                    has_children = true;
                    let Ok(entry) = entry else {
                        continue;
                    };
                    let child = entry.path();
                    if is_link(&child) || !child.is_dir() {
                        continue;
                    }
                    let text = child.to_string_lossy().to_string();
                    if !state.matcher.is_ignored(&text) {
                        deferred.push(text);
                    }
                }
            }
            Err(_) => {
                has_children = true;
            }
        }

        state.counters.increment_visited();
        if !has_children {
            state.counters.increment_empty();
            let modified = std::fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .map(timestamp::format_utc_iso)
                .unwrap_or_default();
            let entry = FolderEntry {
                path: current.clone(),
                modified_utc: modified,
            };
            if state.sender.send(entry).is_err() {
                return;
            }
            continue;
        }

        for value in deferred.into_iter().rev() {
            pending.push(value);
        }
    }
}

fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(true)
}

pub fn scan_roots_of(
    root: &str,
    ignored_paths: &[String],
    patterns: &[String],
) -> AppResult<Vec<String>> {
    let matcher = IgnoreMatcher::new(ignored_paths, patterns);
    enumerate_scan_roots(root, &matcher)
}

pub fn collect_empty_directories(
    root: &str,
    ignored_paths: &[String],
    patterns: &[String],
) -> AppResult<Vec<String>> {
    let matcher = IgnoreMatcher::new(ignored_paths, patterns);
    let mut result: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut pending: Vec<String> = enumerate_scan_roots(root, &matcher)?;
    while let Some(current) = pending.pop() {
        if path_guard::get_depth(&current) > MAX_DIRECTORY_DEPTH {
            continue;
        }
        let path = PathBuf::from(&current);
        if !path.is_dir() || is_link(&path) {
            continue;
        }
        let mut has_children = false;
        let mut deferred: Vec<String> = Vec::new();
        match std::fs::read_dir(&path) {
            Ok(entries) => {
                for entry in entries {
                    has_children = true;
                    let Ok(entry) = entry else {
                        continue;
                    };
                    let child = entry.path();
                    if is_link(&child) || !child.is_dir() {
                        continue;
                    }
                    let text = child.to_string_lossy().to_string();
                    if !matcher.is_ignored(&text) {
                        deferred.push(text);
                    }
                }
            }
            Err(_) => {
                has_children = true;
            }
        }

        if !has_children {
            if seen.insert(current.to_lowercase()) {
                result.push(current);
            }
            continue;
        }

        for value in deferred.into_iter().rev() {
            pending.push(value);
        }
    }
    Ok(result)
}
