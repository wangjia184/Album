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

/// A window of the queue starting at a normalized cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueWindow {
    pub images: Vec<String>,
    pub next_offset: u64,
    pub done: bool,
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

    /// Cursor window: `offset % len` start, wrap-inside for `limit`,
    /// `next_offset = (start + 1) % len`. Empty queue never takes `% 0`.
    pub fn window(&self, offset: u64, limit: usize) -> QueueWindow {
        let images = self.images.lock().expect("queue mutex");
        let done = self.done.load(Ordering::Acquire);
        let len = images.len();
        if len == 0 {
            return QueueWindow {
                images: Vec::new(),
                next_offset: 0,
                done,
            };
        }
        let len64 = len as u64;
        let start = (offset % len64) as usize;
        let slice = (0..limit)
            .map(|i| images[((start as u64 + i as u64) % len64) as usize].clone())
            .collect();
        QueueWindow {
            images: slice,
            next_offset: ((start as u64 + 1) % len64),
            done,
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
/// Symlinked entries (dirs and files) are skipped, unreadable dirs are
/// skipped; `tx` is dropped when the walk finishes.
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

    #[test]
    fn scan_collects_only_images_nested() {
        let tmp = fixture();
        let q = ImageQueue::start(tmp.path().to_path_buf());
        q.wait_until_ready();
        let w = q.window(0, 3);
        let mut got = w.images.clone();
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
        let w = q.window(0, 100);
        assert!(
            w.images.is_empty(),
            "symlinked dir was followed: {:?}",
            w.images
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
        let mid = q.window(0, 8);
        assert_eq!(mid.images.len(), 8, "batch not received");
        assert_ne!(
            mid.images, sent,
            "timeout+dirty shuffle did not run while sender open"
        );
        let mut a = mid.images.clone();
        let mut b = sent.clone();
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
        let w = q.window(0, 10);
        assert!(w.images.is_empty(), "unexpected data: {:?}", w.images);
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
    fn from_paths_window_wrap_and_next() {
        let q = ImageQueue::from_paths(["a", "b", "c"].map(String::from).to_vec());

        let w = q.window(2, 2);
        assert_eq!(w.images, ["c", "a"]);
        assert_eq!(w.next_offset, 0);
        assert!(w.done);

        let w = q.window(3, 2);
        assert_eq!(w.images, ["a", "b"]);
        assert_eq!(w.next_offset, 1);

        let w = q.window(0, 0);
        assert!(w.images.is_empty());
        assert_eq!(w.next_offset, 1);
    }

    #[test]
    fn from_paths_empty_window() {
        let q = ImageQueue::from_paths(Vec::new());
        let w = q.window(u64::MAX, 12);
        assert!(w.images.is_empty());
        assert_eq!(w.next_offset, 0);
        assert!(w.done, "empty seeded queue counts as finished");
    }
}
