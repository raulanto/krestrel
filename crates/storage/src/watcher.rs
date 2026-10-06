//! File system watcher for external changes, Git updates, and merge conflict detection.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Removed(PathBuf),
    MergeConflict(PathBuf),
    BatchReload,
}

pub struct CollectionWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<FileChangeEvent>,
}

impl CollectionWatcher {
    pub fn watch(path: impl AsRef<Path>) -> Result<Self, notify::Error> {
        let (tx_out, rx_out) = channel();
        let (tx_in, rx_in) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx_in.send(event);
                }
            },
            Config::default(),
        )?;

        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

        // Debounce & batch worker thread
        thread::spawn(move || {
            let debounce_window = Duration::from_millis(150);
            let mut pending_paths: Vec<PathBuf> = Vec::new();
            let mut last_event_time = Instant::now();

            loop {
                match rx_in.recv_timeout(Duration::from_millis(50)) {
                    Ok(event) => {
                        last_event_time = Instant::now();
                        for path in event.paths {
                            // Filter out temp files and hidden files
                            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            if filename.ends_with(".tmp") || filename.starts_with('.') {
                                continue;
                            }

                            match event.kind {
                                EventKind::Modify(_)
                                | EventKind::Create(_)
                                | EventKind::Remove(_)
                                    if !pending_paths.contains(&path) =>
                                {
                                    pending_paths.push(path);
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if !pending_paths.is_empty() && last_event_time.elapsed() >= debounce_window
                        {
                            if pending_paths.len() > 10 {
                                // Batch reload for mass git operations (git checkout / git pull)
                                let _ = tx_out.send(FileChangeEvent::BatchReload);
                            } else {
                                for path in pending_paths.drain(..) {
                                    if path.exists() {
                                        if let Ok(content) = fs::read_to_string(&path)
                                            && has_git_merge_conflicts(&content)
                                        {
                                            let _ =
                                                tx_out.send(FileChangeEvent::MergeConflict(path));
                                            continue;
                                        }
                                        let _ = tx_out.send(FileChangeEvent::Modified(path));
                                    } else {
                                        let _ = tx_out.send(FileChangeEvent::Removed(path));
                                    }
                                }
                            }
                            pending_paths.clear();
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            rx: rx_out,
        })
    }

    /// Tries to receive the next pending file change event without blocking.
    pub fn poll_event(&self) -> Option<FileChangeEvent> {
        self.rx.try_recv().ok()
    }
}

/// Detects Git merge conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) in text content.
pub fn has_git_merge_conflicts(content: &str) -> bool {
    let mut has_start = false;
    let mut has_mid = false;
    let mut has_end = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<<<<<<<") {
            has_start = true;
        } else if trimmed.starts_with("=======") {
            has_mid = true;
        } else if trimmed.starts_with(">>>>>>>") {
            has_end = true;
        }
    }

    has_start && (has_mid || has_end)
}
