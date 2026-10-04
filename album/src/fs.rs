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

/// Synology `@eaDir` thumbnail files, largest → smallest.
const THUMB_SIZES: [&str; 7] = ["XL", "L", "M", "SM", "B", "S", "PREVIEW"];

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

/// True when `name` classifies as an image (single policy home for exts).
pub(crate) fn is_image_path(name: &str) -> bool {
    kind_from_name(name) == ChildKind::Image
}

/// True for entries the browser must never surface: Synology metadata dirs
/// (`@eaDir`, `@recycle`, …) and dotfiles (`.DS_Store`, …).
pub(crate) fn is_ignored_name(name: &str) -> bool {
    name.starts_with('@') || name.starts_with('.')
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
            if is_ignored_name(&name) {
                continue;
            }
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

    /// Random pick of up to `n` image children of `rel`. Uses direct children
    /// when any exist; otherwise falls back to images in subdirectories down to
    /// 3 levels, returned as relative subpaths (`sub/img.jpg`).
    pub fn random_images(&self, rel: &str, n: usize) -> io::Result<Vec<String>> {
        let dir = self.resolve(rel)?;
        if !dir.is_dir() {
            return Err(not_found("not a directory"));
        }
        let mut images = Self::direct_images(&dir, "")?;
        if images.is_empty() {
            images = Self::collect_nested(&dir, "", 3);
        }
        for i in 0..n.min(images.len()) {
            let j = fastrand::usize(i..images.len());
            images.swap(i, j);
        }
        images.truncate(n);
        Ok(images)
    }

    /// Image files directly inside `dir`; names carry the `prefix` path.
    fn direct_images(dir: &Path, prefix: &str) -> io::Result<Vec<String>> {
        let mut images = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_ignored_name(&name) {
                continue;
            }
            if kind_from_name(&name) == ChildKind::Image {
                images.push(format!("{prefix}{name}"));
            }
        }
        Ok(images)
    }

    /// Images inside subdirectories of `dir`, descending at most `levels` more
    /// directory levels. Best-effort: unreadable entries are skipped, and
    /// symlinks are not followed (`file_type` reports links as links).
    fn collect_nested(dir: &Path, prefix: &str, levels: usize) -> Vec<String> {
        if levels == 0 {
            return Vec::new();
        }
        let mut images = Vec::new();
        let Ok(entries) = fs::read_dir(dir) else {
            return images;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_ignored_name(&name) {
                continue;
            }
            let sub_prefix = format!("{prefix}{name}/");
            if let Ok(found) = Self::direct_images(&entry.path(), &sub_prefix) {
                images.extend(found);
            }
            images.extend(Self::collect_nested(&entry.path(), &sub_prefix, levels - 1));
        }
        images
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

    /// Resolve a confined regular file's absolute path (no open). Used for
    /// metadata/exif reads that need the path itself.
    pub fn resolved_file_path(&self, rel: &str) -> io::Result<PathBuf> {
        let path = self.resolve(rel)?;
        if !path.is_file() {
            return Err(not_found("not a file"));
        }
        Ok(path)
    }

    /// True when `rel` resolves to a regular file under the mount root.
    fn is_file_at(&self, rel: &str) -> bool {
        self.resolve(rel).is_ok_and(|p| p.is_file())
    }

    /// Synology thumbnail for `rel`: `<parent>/@eaDir/<name>` where the
    /// entry is either a directory of `SYNOPHOTO_THUMB_{size}.jpg` files
    /// (tried XL→…→PREVIEW) or a single flat file; falls back to `rel`
    /// itself when no thumbnail exists.
    pub fn thumbnail_rel(&self, rel: &str) -> String {
        let (parent, name) = match rel.rsplit_once('/') {
            Some((p, n)) => (p, n),
            None => ("", rel),
        };
        let ea = if parent.is_empty() {
            format!("@eaDir/{name}")
        } else {
            format!("{parent}/@eaDir/{name}")
        };
        for size in THUMB_SIZES {
            let cand = format!("{ea}/SYNOPHOTO_THUMB_{size}.jpg");
            if self.is_file_at(&cand) {
                return cand;
            }
        }
        if self.is_file_at(&ea) {
            return ea;
        }
        rel.to_string()
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
        // Synology PhotoStation metadata dir + macOS junk: must never be listed.
        fs::create_dir(root.join("@eaDir")).expect("@eaDir");
        fs::write(root.join("@eaDir").join("z_file.txt"), "thumb").expect("@eaDir thumb");
        fs::write(root.join("@eaDir").join("a_file.jpg"), "thumb").expect("@eaDir thumb jpg");
        fs::write(root.join(".DS_Store"), b"ds").expect(".DS_Store");
        tmp
    }

    #[test]
    fn list_skips_at_and_dot_entries() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        let children = album.list_children("").expect("list root");
        for c in &children {
            assert!(
                !c.name.starts_with('@') && !c.name.starts_with('.'),
                "unexpected hidden/synology entry {:?}",
                c.name
            );
        }
        let names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
        assert!(!names.contains(&"@eaDir"));
        assert!(!names.contains(&".DS_Store"));
    }

    #[test]
    fn random_images_skips_at_and_dot_entries() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        // a_dir only contains nested.txt → falls back to nesting; @eaDir's
        // a_file.jpg must never be picked, and .DS_Store isn't an image anyway.
        let picked = album.random_images("", 100).expect("random_images");
        for p in &picked {
            assert!(
                !p.contains("@eaDir") && !p.starts_with('.'),
                "unexpected pick {p}"
            );
        }
        assert!(picked.contains(&"a_file.jpg".to_string()), "real image expected");
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
            .open_file("escape_link/etc/passwd")
            .expect_err("open escape must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn random_images_picks_up_to_n_direct_images() {
        let tmp = fixture();
        let root = tmp.path().join("pics");
        fs::create_dir(&root).expect("pics");
        for name in ["a.jpg", "b.png", "c.gif", "d.webp", "clip.mp4", "note.txt"] {
            fs::write(root.join(name), b"x").expect(name);
        }
        let album = AlbumFs::new(tmp.path()).expect("new");

        let picked = album.random_images("pics", 3).expect("random_images");
        assert_eq!(picked.len(), 3);
        let allowed = ["a.jpg", "b.png", "c.gif", "d.webp"];
        for name in &picked {
            assert!(allowed.contains(&name.as_str()), "unexpected {name}");
        }
        let mut deduped = picked.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(deduped.len(), 3, "picks must be unique");
    }

    #[test]
    fn random_images_returns_all_when_fewer_than_n() {
        let tmp = fixture();
        let root = tmp.path().join("pics");
        fs::create_dir(&root).expect("pics");
        for name in ["only_one.jpg", "also_two.jpg"] {
            fs::write(root.join(name), b"x").expect(name);
        }
        let album = AlbumFs::new(tmp.path()).expect("new");

        let picked = album.random_images("pics", 3).expect("random_images");
        assert_eq!(picked.len(), 2);
    }

    #[test]
    fn random_images_no_direct_images_is_empty() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");

        let picked = album.random_images("a_dir", 3).expect("random_images");
        assert!(picked.is_empty());
    }

    #[test]
    fn random_images_missing_dir_is_err() {
        let tmp = fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        let err = album
            .random_images("no_such_dir", 3)
            .expect_err("missing dir must fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn random_images_recurses_when_no_direct_images() {
        let tmp = fixture();
        let root = tmp.path().join("album");
        fs::create_dir(&root).expect("album");
        fs::create_dir(root.join("sub1")).expect("sub1");
        fs::create_dir(root.join("sub2")).expect("sub2");
        fs::write(root.join("sub1").join("x.jpg"), b"x").expect("x.jpg");
        fs::write(root.join("sub2").join("y.jpg"), b"x").expect("y.jpg");
        let album = AlbumFs::new(tmp.path()).expect("new");

        let picked = album.random_images("album", 3).expect("random_images");
        assert_eq!(picked.len(), 2, "nested images must be found");
        for name in &picked {
            assert!(name.contains('/'), "expected relative subpath, got {name}");
        }
    }

    #[test]
    fn random_images_recursion_stops_after_three_levels() {
        let tmp = fixture();
        let l3 = tmp.path().join("deep").join("a").join("b").join("c");
        let l4 = l3.join("d");
        fs::create_dir_all(&l3).expect("l3");
        fs::create_dir_all(&l4).expect("l4");
        fs::write(l3.join("img3.jpg"), b"x").expect("img3");
        fs::write(l4.join("img4.jpg"), b"x").expect("img4");
        let album = AlbumFs::new(tmp.path()).expect("new");

        let picked = album.random_images("deep", 10).expect("random_images");
        assert_eq!(
            picked,
            vec!["a/b/c/img3.jpg".to_string()],
            "depth-3 image found, depth-4 must be out of reach"
        );
    }

    #[test]
    fn random_images_prefers_direct_over_nested() {
        let tmp = fixture();
        let root = tmp.path().join("album");
        fs::create_dir(&root).expect("album");
        fs::create_dir(root.join("sub")).expect("sub");
        fs::write(root.join("direct.jpg"), b"x").expect("direct");
        fs::write(root.join("sub").join("nested.jpg"), b"x").expect("nested");
        let album = AlbumFs::new(tmp.path()).expect("new");

        let picked = album.random_images("album", 10).expect("random_images");
        assert_eq!(picked, vec!["direct.jpg".to_string()]);
    }

    /// Fixture for thumbnail resolution: dir-style (SYNOPHOTO_THUMB_*),
    /// flat-style (`@eaDir/<name>` file), mixed-case name, and no-thumb file.
    fn thumb_fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().expect("tempdir");
        let pics = tmp.path().join("pics");
        fs::create_dir_all(pics.join("@eaDir/dirstyle.jpg")).expect("dirstyle eaDir");
        fs::write(pics.join("dirstyle.jpg"), b"original").expect("dirstyle orig");
        fs::write(
            pics.join("@eaDir/dirstyle.jpg").join("SYNOPHOTO_THUMB_M.jpg"),
            b"thumb-m",
        )
        .expect("thumb m");
        fs::write(
            pics.join("@eaDir/dirstyle.jpg").join("SYNOPHOTO_THUMB_XL.jpg"),
            b"thumb-xl",
        )
        .expect("thumb xl");
        fs::create_dir_all(pics.join("@eaDir")).expect("eaDir dir");
        fs::write(pics.join("@eaDir/flat.jpg"), b"thumb-flat").expect("flat thumb");
        fs::write(pics.join("flat.jpg"), b"original").expect("flat orig");
        fs::create_dir_all(pics.join("@eaDir/UPPER.JPG")).expect("upper eaDir");
        fs::write(
            pics.join("@eaDir/UPPER.JPG").join("SYNOPHOTO_THUMB_L.jpg"),
            b"thumb-l",
        )
        .expect("thumb l");
        fs::write(pics.join("UPPER.JPG"), b"original").expect("upper orig");
        fs::write(pics.join("nothumb.jpg"), b"original").expect("nothumb orig");
        fs::write(pics.join("nothumb.jpg@SynoEAStream"), b"").expect("stream sidecar");
        tmp
    }

    #[test]
    fn thumbnail_rel_prefers_dir_style_xl_then_size_order() {
        let tmp = thumb_fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        assert_eq!(
            album.thumbnail_rel("pics/dirstyle.jpg"),
            "pics/@eaDir/dirstyle.jpg/SYNOPHOTO_THUMB_XL.jpg"
        );
    }

    #[test]
    fn thumbnail_rel_uses_flat_eaDir_file() {
        let tmp = thumb_fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        assert_eq!(album.thumbnail_rel("pics/flat.jpg"), "pics/@eaDir/flat.jpg");
    }

    #[test]
    fn thumbnail_rel_keeps_exact_case() {
        let tmp = thumb_fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        assert_eq!(
            album.thumbnail_rel("pics/UPPER.JPG"),
            "pics/@eaDir/UPPER.JPG/SYNOPHOTO_THUMB_L.jpg"
        );
    }

    #[test]
    fn thumbnail_rel_falls_back_to_original_when_missing() {
        let tmp = thumb_fixture();
        let album = AlbumFs::new(tmp.path()).expect("new");
        assert_eq!(album.thumbnail_rel("pics/nothumb.jpg"), "pics/nothumb.jpg");
        // Video-ish / any path without thumbnails anywhere: unchanged.
        assert_eq!(album.thumbnail_rel("pics/nothumb.jpg"), "pics/nothumb.jpg");
    }

    #[test]
    fn thumbnail_rel_root_level_no_parent() {
        let tmp = thumb_fixture();
        let root = tmp.path();
        fs::create_dir(root.join("@eaDir")).expect("@eaDir root");
        fs::write(root.join("@eaDir/top.jpg"), b"thumb").expect("top thumb");
        fs::write(root.join("top.jpg"), b"original").expect("top orig");
        let album = AlbumFs::new(root).expect("new");
        assert_eq!(album.thumbnail_rel("top.jpg"), "@eaDir/top.jpg");
    }
}
