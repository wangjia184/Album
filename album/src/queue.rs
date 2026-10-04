use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// One mount's shuffled playback queue: filled progressively by a scanner
/// thread over `mpsc`, maintained by a collector thread, read lock-light by
/// HTTP handlers.
#[derive(Debug)]
pub struct ImageQueue {
    images: Mutex<Vec<String>>,
    done: AtomicBool,
}

/// One queue entry: absolute index in the shuffled array + mount-relative path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QueueItem {
    pub index: u64,
    pub path: String,
}

/// A window of the queue: `limit` items (wrap-inside) plus the scan `done`
/// flag and the full queue length (clients wrap display↔queue indices).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueWindow {
    pub items: Vec<QueueItem>,
    pub done: bool,
    pub total: usize,
}

impl ImageQueue {
    /// Spawn scanner + collector for `root` with the default recv timeout.
    pub fn start(root: PathBuf) -> Arc<Self> {
        Self::start_with_timeout(root, Duration::from_secs(2))
    }

    /// Spawn scanner + collector; `timeout` is the collector's recv_timeout
    /// (injected in tests).
    pub fn start_with_timeout(root: PathBuf, timeout: Duration) -> Arc<Self> {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let q = Arc::new(Self {
            images: Mutex::new(Vec::new()),
            done: AtomicBool::new(false),
        });
        let qc = Arc::clone(&q);
        std::thread::spawn(move || run_collector(rx, &qc, timeout));
        std::thread::spawn(move || scan(&root, &tx));
        q
    }

    /// Pre-seeded, already-finished queue (test seam).
    pub fn from_paths(paths: Vec<String>) -> Arc<Self> {
        let q = Arc::new(Self {
            images: Mutex::new(paths),
            done: AtomicBool::new(false),
        });
        q.done.store(true, Ordering::Release);
        q
    }

    /// Cursor window over the shuffled array. Empty queue never takes `% 0`.
    ///
    /// - `center=false`: starts at `offset % len` (forward window).
    /// - `center=true`: `offset` names the *current* photo; it lands at slot
    ///   `mid = (limit-1)/2`, with `floor_mod` wrapping so `offset < mid`
    ///   never underflows. Client never sends negative offsets.
    pub fn window(&self, offset: u64, limit: usize, center: bool) -> QueueWindow {
        let images = self.images.lock().expect("queue mutex");
        let done = self.done.load(Ordering::Acquire);
        let len = images.len();
        if len == 0 {
            return QueueWindow {
                items: Vec::new(),
                done,
                total: 0,
            };
        }
        let len64 = len as u64;
        let start = if center {
            let mid = (limit.saturating_sub(1)) / 2;
            let o = (offset % len64) as i64;
            o.wrapping_sub(mid as i64).rem_euclid(len as i64) as usize % len
        } else {
            (offset % len64) as usize
        };
        let items = (0..limit)
            .map(|i| {
                let idx_us = (start + i) % len;
                QueueItem {
                    index: idx_us as u64,
                    path: images[idx_us].clone(),
                }
            })
            .collect();
        QueueWindow {
            items,
            done,
            total: len,
        }
    }

    /// Spin until the collector marks the scan finished (test seam; 10s cap).
    pub fn wait_until_ready(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.done.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline, "queue not ready within 10s");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

/// Iterative walk of `root`; sends `/`-joined image rel paths on `tx`.
/// Symlinked entries (dirs and files) and `@`/`.` names (Synology `@eaDir`,
/// dotfiles) are skipped, unreadable dirs are skipped; `tx` is dropped when
/// the walk finishes.
fn scan(root: &Path, tx: &Sender<String>) {
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(err) => {
                tracing::debug!("queue scan skips unreadable {}: {err}", dir.display());
                continue;
            }
        };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if crate::fs::is_ignored_name(&name) {
                continue;
            }
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_symlink() {
                continue;
            }
            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            if ft.is_dir() {
                stack.push((entry.path(), rel));
            } else if ft.is_file() && crate::fs::is_image_path(&name) {
                let _ = tx.send(rel);
            }
        }
    }
}

fn fisher_yates(images: &mut [String]) {
    for i in (1..images.len()).rev() {
        let j = fastrand::usize(..=i);
        images.swap(i, j);
    }
}

/// Collect scanned paths: append while arriving; shuffle a dirty batch after
/// a quiet timeout; final shuffle on disconnect; then mark `done`.
fn run_collector(rx: Receiver<String>, q: &ImageQueue, timeout: Duration) {
    let mut dirty = false;
    loop {
        match rx.recv_timeout(timeout) {
            Ok(path) => {
                {
                    let mut images = q.images.lock().expect("queue mutex");
                    images.push(path);
                    while let Ok(more) = rx.try_recv() {
                        images.push(more);
                    }
                }
                dirty = true;
            }
            Err(RecvTimeoutError::Timeout) => {
                if dirty {
                    let mut images = q.images.lock().expect("queue mutex");
                    if images.len() > 1 {
                        fisher_yates(&mut images);
                    }
                    dirty = false;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                if dirty {
                    let mut images = q.images.lock().expect("queue mutex");
                    if images.len() > 1 {
                        fisher_yates(&mut images);
                    }
                }
                q.done.store(true, Ordering::Release);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::write(root.join("a.jpg"), b"j").expect("a.jpg");
        std::fs::create_dir(root.join("sub")).expect("sub");
        std::fs::write(root.join("sub").join("b.png"), b"p").expect("b.png");
        std::fs::create_dir(root.join("sub").join("deep")).expect("deep");
        std::fs::write(root.join("sub").join("deep").join("c.jpg"), b"j").expect("c.jpg");
        std::fs::write(root.join("x.txt"), b"t").expect("x.txt");
        std::fs::write(root.join("m.mp4"), b"v").expect("m.mp4");
        tmp
    }

    fn paths(w: &QueueWindow) -> Vec<&str> {
        w.items.iter().map(|i| i.path.as_str()).collect()
    }

    fn indices(w: &QueueWindow) -> Vec<u64> {
        w.items.iter().map(|i| i.index).collect()
    }

    #[test]
    fn scan_collects_only_images_nested() {
        let tmp = fixture();
        let q = ImageQueue::start(tmp.path().to_path_buf());
        q.wait_until_ready();
        let w = q.window(0, 3, false);
        let mut got = paths(&w);
        got.sort();
        assert_eq!(got, ["a.jpg", "sub/b.png", "sub/deep/c.jpg"]);
        assert!(w.done);
    }

    #[cfg(unix)]
    #[test]
    fn scan_skips_symlinked_dirs() {
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("img.jpg"), b"j").expect("img.jpg");
        let tmp = tempfile::tempdir().expect("root");
        std::os::unix::fs::symlink(outside.path(), tmp.path().join("link")).expect("symlink");
        let q = ImageQueue::start(tmp.path().to_path_buf());
        q.wait_until_ready();
        let w = q.window(0, 100, false);
        assert!(
            w.items.is_empty(),
            "symlinked dir was followed: {:?}",
            paths(&w)
        );
    }

    #[test]
    fn collector_timeout_shuffles_dirty_once() {
        fastrand::seed(1);
        let q = Arc::new(ImageQueue {
            images: Mutex::new(Vec::new()),
            done: AtomicBool::new(false),
        });
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let qc = Arc::clone(&q);
        let collector =
            std::thread::spawn(move || run_collector(rx, &qc, Duration::from_millis(50)));

        let sent: Vec<String> = (0..8).map(|i| format!("p{i}.jpg")).collect();
        for p in &sent {
            tx.send(p.clone()).expect("send");
        }
        // Sender still open: the 50ms recv_timeout must fire and shuffle the dirty batch.
        std::thread::sleep(Duration::from_millis(250));
        let mid = q.window(0, 8, false);
        let got = paths(&mid);
        assert_eq!(got.len(), 8, "batch not received");
        let sent_refs: Vec<&str> = sent.iter().map(String::as_str).collect();
        assert_ne!(
            got, sent_refs,
            "timeout+dirty shuffle did not run while sender open"
        );
        let mut a = got;
        let mut b = sent_refs;
        a.sort();
        b.sort();
        assert_eq!(a, b, "shuffle must be a permutation");

        drop(tx);
        collector.join().expect("collector thread");
        assert!(
            q.done.load(Ordering::Acquire),
            "done not set after disconnect"
        );
    }

    #[test]
    fn collector_not_dirty_timeout_waits_then_done() {
        let q = Arc::new(ImageQueue {
            images: Mutex::new(Vec::new()),
            done: AtomicBool::new(false),
        });
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let qc = Arc::clone(&q);
        let collector =
            std::thread::spawn(move || run_collector(rx, &qc, Duration::from_millis(50)));

        std::thread::sleep(Duration::from_millis(200));
        let w = q.window(0, 10, false);
        assert!(w.items.is_empty(), "unexpected data: {:?}", paths(&w));
        assert!(
            !q.done.load(Ordering::Acquire),
            "done set while sender open"
        );

        drop(tx);
        collector.join().expect("collector thread");
        assert!(
            q.done.load(Ordering::Acquire),
            "done not set after disconnect"
        );
    }

    #[test]
    fn forward_window_items_and_indices() {
        let q = ImageQueue::from_paths(["a", "b", "c"].map(String::from).to_vec());

        let w = q.window(2, 2, false);
        assert_eq!(paths(&w), ["c", "a"]);
        assert_eq!(indices(&w), [2, 0]);
        assert!(w.done);
    }

    #[test]
    fn empty_window_is_empty_even_with_center() {
        let q = ImageQueue::from_paths(Vec::new());
        let w = q.window(u64::MAX, 12, true);
        assert!(w.items.is_empty());
        assert!(w.done, "empty seeded queue counts as finished");
    }

    #[test]
    fn center_places_offset_at_mid_odd_limit() {
        // len=7, offset=5, limit=5 → mid=2, start=(5-2)=3 → indices [3,4,5,6,0]
        let pool: Vec<String> = (0..7).map(|i| format!("p{i}")).collect();
        let q = ImageQueue::from_paths(pool);
        let w = q.window(5, 5, true);
        assert_eq!(w.items.len(), 5);
        assert_eq!(w.items[2].index, 5, "offset must land at mid");
        assert_eq!(indices(&w), [3, 4, 5, 6, 0]);
    }

    #[test]
    fn center_places_offset_at_mid_even_limit() {
        // len=10, offset=3, limit=10 → mid=4, start=(3-4) rem_euclid 10 = 9
        let pool: Vec<String> = (0..10).map(|i| format!("p{i}")).collect();
        let q = ImageQueue::from_paths(pool);
        let w = q.window(3, 10, true);
        assert_eq!(w.items.len(), 10);
        assert_eq!(w.items[4].index, 3, "offset must land at mid");
        assert_eq!(indices(&w), [9, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn center_offset_below_mid_wraps() {
        // len=3, offset=0, limit=5 → mid=2, start=(0-2) rem_euclid 3 = 1
        let pool: Vec<String> = (0..3).map(|i| format!("p{i}")).collect();
        let q = ImageQueue::from_paths(pool);
        let w = q.window(0, 5, true);
        assert_eq!(w.items.len(), 5);
        assert_eq!(w.items[2].index, 0, "offset must land at mid after wrap");
        assert_eq!(indices(&w), [1, 2, 0, 1, 2]);
    }

    #[test]
    fn center_index_sequence_is_contiguous() {
        let pool: Vec<String> = (0..7).map(|i| format!("p{i}")).collect();
        let q = ImageQueue::from_paths(pool);
        let w = q.window(5, 9, true); // limit=9 → mid=4 → start=(5-4)=1; wraps > once
        let expected: Vec<u64> = (0..9).map(|i| ((1 + i) % 7) as u64).collect();
        assert_eq!(indices(&w), expected);
        // paths must match their indices
        for item in &w.items {
            assert_eq!(item.path, format!("p{}", item.index));
        }
    }
}
