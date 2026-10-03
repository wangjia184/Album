use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Immutable, canonical absolute root for one mount.
#[derive(Debug, Clone)]
pub struct AlbumFs {
    root: PathBuf,
}

/// Classification of a direct child, serialized as lowercase `dir|image|video|other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChildKind {
    Dir,
    Image,
    Video,
    Other,
}

/// One direct child of a listed directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListedChild {
    pub name: String,
    pub kind: ChildKind,
}

const IMAGE_EXTS: [&str; 7] = ["jpg", "jpeg", "png", "gif", "webp", "bmp", "avif"];
const VIDEO_EXTS: [&str; 6] = ["mp4", "mov", "mkv", "webm", "avi", "m4v"];

fn not_found(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, msg)
}

fn kind_from_name(name: &str) -> ChildKind {
    let ext = Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if IMAGE_EXTS.contains(&ext.as_str()) {
        ChildKind::Image
    } else if VIDEO_EXTS.contains(&ext.as_str()) {
        ChildKind::Video
    } else {
        ChildKind::Other
    }
}

impl AlbumFs {
    /// Canonicalize `root` at construction; fail if not a directory.
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = fs::canonicalize(root.as_ref())?;
        if !root.is_dir() {
            return Err(not_found("root is not a directory"));
        }
        Ok(Self { root })
    }

    /// Basename of the mount root (e.g. `/workspace/.www` → `.www`).
    /// Returns `""` for `/` or when the path has no final component.
    pub fn root_name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Parse `rel` into a safe relative path: reject absolute, `.`, `..`,
    /// empty segments, and interior NUL; empty `rel` maps to root.
    fn parse_rel(rel: &str) -> io::Result<PathBuf> {
        if rel.contains('\0') {
            return Err(not_found("nul byte in path"));
        }
        if rel.is_empty() {
            return Ok(PathBuf::new());
        }
        if rel.starts_with('/') || Path::new(rel).is_absolute() {
            return Err(not_found("absolute path"));
        }
        let mut out = PathBuf::new();
        for seg in rel.split('/') {
            match seg {
                "" | "." | ".." => return Err(not_found("illegal path segment")),
                s => out.push(s),
            }
        }
        Ok(out)
    }

    /// Join `root` + `rel`, canonicalize, and require the result stays under root.
    fn resolve(&self, rel: &str) -> io::Result<PathBuf> {
        let rel_path = Self::parse_rel(rel)?;
        let joined = self.root.join(rel_path);
        let canonical = fs::canonicalize(&joined)?;
        if !canonical.starts_with(&self.root) {
            return Err(not_found("path escapes root"));
        }
        Ok(canonical)
    }

    /// Direct children only. `rel` is a relative path; empty = root.
    /// Sort: dirs by name, then files by name.
    pub fn list_children(&self, rel: &str) -> io::Result<Vec<ListedChild>> {
        let dir = self.resolve(rel)?;
        if !dir.is_dir() {
            return Err(not_found("not a directory"));
        }
        let mut children = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let is_dir = entry.file_type()?.is_dir();
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = if is_dir {
                ChildKind::Dir
            } else {
                kind_from_name(&name)
            };
            children.push(ListedChild { name, kind });
        }
        children.sort_by(|a, b| {
            let a_dir = a.kind == ChildKind::Dir;
            let b_dir = b.kind == ChildKind::Dir;
            b_dir.cmp(&a_dir).then_with(|| a.name.cmp(&b.name))
        });
        Ok(children)
    }

    /// Open a confined regular file for reading. Directories and escapes
    /// yield `io::ErrorKind::NotFound`.
    pub fn open_file(&self, rel: &str) -> io::Result<File> {
        let path = self.resolve(rel)?;
        if !path.is_file() {
            return Err(not_found("not a file"));
        }
        File::open(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir(root.join("b_dir")).expect("b_dir");
        fs::create_dir(root.join("a_dir")).expect("a_dir");
        fs::write(root.join("a_dir").join("nested.txt"), "nested contents").expect("nested.txt");
        fs::write(root.join("z_file.txt"), "z file contents").expect("z_file.txt");
        fs::write(root.join("a_file.jpg"), b"jpg").expect("a_file.jpg");
        fs::write(root.join("m_clip.mp4"), b"mp4").expect("m_clip.mp4");
        fs::write(root.join("other.bin"), b"bin").expect("other.bin");
        tmp
    }

    #[test]
    fn new_rejects_missing_root() {
        let tmp = fixture();
        let missing = tmp.path().join("no_such_dir");
        let err = AlbumFs::new(missing).expect_err("missing root must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn list_root_sorts_dirs_then_files() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        let children = album.list_children("").expect("list root");
        let names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "a_dir",
                "b_dir",
                "a_file.jpg",
                "m_clip.mp4",
                "other.bin",
                "z_file.txt"
            ]
        );
        let kinds: Vec<ChildKind> = children.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            [
                ChildKind::Dir,
                ChildKind::Dir,
                ChildKind::Image,
                ChildKind::Video,
                ChildKind::Other,
                ChildKind::Other
            ]
        );
        assert_eq!(serde_json::to_string(&kinds[0]).unwrap(), "\"dir\"");
        assert_eq!(serde_json::to_string(&kinds[2]).unwrap(), "\"image\"");
        assert_eq!(serde_json::to_string(&kinds[3]).unwrap(), "\"video\"");
        assert_eq!(serde_json::to_string(&kinds[4]).unwrap(), "\"other\"");
    }

    #[test]
    fn list_nested_relative_path() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        let children = album.list_children("a_dir").expect("list nested");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "nested.txt");
        assert_eq!(children[0].kind, ChildKind::Other);
    }

    #[test]
    fn list_rejects_dotdot() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        let err = album.list_children("../x").expect_err("dotdot must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn list_rejects_absolute() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        let err = album.list_children("/etc").expect_err("absolute must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn list_symlink_escape_is_err() {
        let tmp = fixture();
        std::os::unix::fs::symlink("/", tmp.path().join("escape_link")).expect("symlink");
        let album = AlbumFs::new(tmp.path()).expect("new");
        let err = album
            .list_children("escape_link")
            .expect_err("symlink escape must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        let err = album
            .list_children("escape_link/etc")
            .expect_err("path under symlink escape must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn open_file_returns_file_with_contents() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        let mut file = album.open_file("z_file.txt").expect("open");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("read");
        assert_eq!(contents, "z file contents");
    }

    #[test]
    fn open_file_on_directory_is_err() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        let err = album.open_file("a_dir").expect_err("open dir must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn open_symlink_escape_is_err() {
        let tmp = fixture();
        std::os::unix::fs::symlink("/", tmp.path().join("escape_link")).expect("symlink");
        let album = AlbumFs::new(tmp.path()).expect("new");
        let err = album
            .open_file("escape_link")
            .expect_err("symlink escape must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        let err = album
            .open_file("escape_link/etc/passwd")
            .expect_err("path under symlink escape must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
