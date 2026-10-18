use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::error::{AppError, AppResult};
use super::ignore_matcher::IgnoreMatcher;
use super::models::{
    DeleteEvent, DeleteOutcome, DeleteProgress, DeleteReport, DeleteRequest, DeleteStage,
};
use super::path_guard;
use super::recycle_bin;

const PROGRESS_INTERVAL: i64 = 8;

pub fn delete<E, P>(
    request: &DeleteRequest,
    cancel: Arc<AtomicBool>,
    mut on_event: E,
    mut on_progress: P,
) -> AppResult<DeleteReport>
where
    E: FnMut(&DeleteEvent),
    P: FnMut(&DeleteProgress),
{
    let Some(root) = path_guard::try_normalize(Some(&request.root_directory)) else {
        return Err(AppError::validation("root_directory_invalid"));
    };

    let matcher = IgnoreMatcher::new(&request.ignored_paths, &request.ignore_patterns);
    let mut queue = build_queue(&request.paths);
    let mut scheduled: HashSet<String> = queue.iter().map(|path| path.to_lowercase()).collect();

    let mut deleted: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let mut remaining: Vec<String> = Vec::new();
    let mut processed: i64 = 0;
    let mut announced_total: i64 = queue.len() as i64;

    on_event(&DeleteEvent {
        stage: DeleteStage::Started,
        outcome: DeleteOutcome::Deleted,
        path: String::new(),
    });

    while !queue.is_empty() {
        if cancel.load(Ordering::SeqCst) {
            remaining.extend(queue.iter().cloned());
            on_event(&DeleteEvent {
                stage: DeleteStage::BatchStopped,
                outcome: DeleteOutcome::Cancelled,
                path: String::new(),
            });
            break;
        }

        let path = queue.remove(0);
        let outcome = try_delete(
            &root,
            &path,
            request.use_recycle_bin,
            &cancel,
            &mut on_event,
        );

        match outcome {
            DeleteOutcome::Deleted | DeleteOutcome::MovedToRecycleBin => {
                deleted.push(path.clone());
                collect_emptied_ancestors(
                    &root,
                    &path,
                    &matcher,
                    &mut queue,
                    &mut scheduled,
                    &mut on_event,
                );
            }
            DeleteOutcome::PermissionDenied
            | DeleteOutcome::Failed
            | DeleteOutcome::OutsideRoot => {
                failed.push(path.clone());
            }
            _ => {
                remaining.push(path.clone());
            }
        }

        processed += 1;
        announced_total = announced_total.max(processed);
        if processed % PROGRESS_INTERVAL == 0 {
            on_progress(&DeleteProgress {
                completed: processed,
                total: announced_total,
            });
        }
    }

    on_progress(&DeleteProgress {
        completed: processed,
        total: announced_total,
    });
    on_event(&DeleteEvent {
        stage: DeleteStage::Completed,
        outcome: DeleteOutcome::Deleted,
        path: String::new(),
    });

    Ok(DeleteReport {
        deleted_paths: deleted,
        failed_paths: failed,
        remaining_paths: remaining,
        was_cancelled: cancel.load(Ordering::SeqCst),
    })
}

pub fn build_queue(paths: &[String]) -> Vec<String> {
    let mut ordered: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for path in paths {
        if let Some(normalized) = path_guard::try_normalize(Some(path)) {
            if seen.insert(normalized.to_lowercase()) {
                ordered.push(normalized);
            }
        }
    }

    ordered.sort_by(|left, right| {
        let by_depth = path_guard::get_depth(right).cmp(&path_guard::get_depth(left));
        if by_depth != std::cmp::Ordering::Equal {
            by_depth
        } else {
            left.to_lowercase().cmp(&right.to_lowercase())
        }
    });
    ordered
}

fn try_delete<E>(
    root: &str,
    path: &str,
    use_recycle_bin: bool,
    cancel: &Arc<AtomicBool>,
    on_event: &mut E,
) -> DeleteOutcome
where
    E: FnMut(&DeleteEvent),
{
    if !path_guard::is_within_root(root, path) || path_guard::is_same_path(root, path) {
        on_event(&DeleteEvent {
            stage: DeleteStage::Finished,
            outcome: DeleteOutcome::OutsideRoot,
            path: path.to_string(),
        });
        return DeleteOutcome::OutsideRoot;
    }

    on_event(&DeleteEvent {
        stage: DeleteStage::Started,
        outcome: DeleteOutcome::Deleted,
        path: path.to_string(),
    });

    if cancel.load(Ordering::SeqCst) {
        on_event(&DeleteEvent {
            stage: DeleteStage::Finished,
            outcome: DeleteOutcome::Cancelled,
            path: path.to_string(),
        });
        return DeleteOutcome::Cancelled;
    }

    let directory = Path::new(path);
    if !directory.is_dir() {
        on_event(&DeleteEvent {
            stage: DeleteStage::Finished,
            outcome: DeleteOutcome::AlreadyGone,
            path: path.to_string(),
        });
        return DeleteOutcome::AlreadyGone;
    }

    if is_link(directory) || has_any_entry(directory) {
        on_event(&DeleteEvent {
            stage: DeleteStage::Finished,
            outcome: DeleteOutcome::NotEmpty,
            path: path.to_string(),
        });
        return DeleteOutcome::NotEmpty;
    }

    if cancel.load(Ordering::SeqCst) {
        on_event(&DeleteEvent {
            stage: DeleteStage::Finished,
            outcome: DeleteOutcome::Cancelled,
            path: path.to_string(),
        });
        return DeleteOutcome::Cancelled;
    }

    if use_recycle_bin && recycle_bin::try_move(path) {
        on_event(&DeleteEvent {
            stage: DeleteStage::Finished,
            outcome: DeleteOutcome::MovedToRecycleBin,
            path: path.to_string(),
        });
        return DeleteOutcome::MovedToRecycleBin;
    }

    match std::fs::remove_dir(directory) {
        Ok(()) => {
            on_event(&DeleteEvent {
                stage: DeleteStage::Finished,
                outcome: DeleteOutcome::Deleted,
                path: path.to_string(),
            });
            DeleteOutcome::Deleted
        }
        Err(error) => {
            let outcome = match error.kind() {
                std::io::ErrorKind::PermissionDenied => DeleteOutcome::PermissionDenied,
                std::io::ErrorKind::NotFound => DeleteOutcome::AlreadyGone,
                _ => DeleteOutcome::Failed,
            };
            on_event(&DeleteEvent {
                stage: DeleteStage::Finished,
                outcome,
                path: path.to_string(),
            });
            outcome
        }
    }
}

fn collect_emptied_ancestors<E>(
    root: &str,
    deleted_path: &str,
    matcher: &IgnoreMatcher,
    queue: &mut Vec<String>,
    scheduled: &mut HashSet<String>,
    on_event: &mut E,
) where
    E: FnMut(&DeleteEvent),
{
    let mut parent = path_guard::get_parent(deleted_path);
    while !parent.is_empty()
        && path_guard::is_within_root(root, &parent)
        && !path_guard::is_same_path(root, &parent)
        && !matcher.is_ignored(&parent)
        && is_empty_directory(&parent)
        && scheduled.insert(parent.to_lowercase())
    {
        on_event(&DeleteEvent {
            stage: DeleteStage::CascadeCandidate,
            outcome: DeleteOutcome::Deleted,
            path: parent.clone(),
        });
        queue.push(parent.clone());
        parent = path_guard::get_parent(&parent);
    }
}

fn has_any_entry(directory: &Path) -> bool {
    match std::fs::read_dir(directory) {
        Ok(mut entries) => entries.next().is_some(),
        Err(_) => true,
    }
}

fn is_empty_directory(path: &str) -> bool {
    let directory = Path::new(path);
    directory.is_dir() && !has_any_entry(directory)
}

fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}
