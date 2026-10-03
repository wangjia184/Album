use std::path::PathBuf;

const STAR: &str = "*";

#[derive(Debug, Clone, Default)]
pub struct MountTable {
    entries: Vec<(String, PathBuf)>,
}

fn normalize_key(key: &str) -> String {
    if key == STAR {
        STAR.to_string()
    } else {
        key.to_ascii_lowercase()
    }
}

impl MountTable {
    pub fn get(&self, hostname: &str) -> Option<&PathBuf> {
        let key = hostname.to_ascii_lowercase();
        self.entries
            .iter()
            .find(|(k, _)| *k == key)
            .or_else(|| self.entries.iter().find(|(k, _)| k == STAR))
            .map(|(_, path)| path)
    }

    /// All configured roots, in mount order (duplicates possible).
    pub fn roots(&self) -> impl Iterator<Item = &PathBuf> {
        self.entries.iter().map(|(_, path)| path)
    }
}

impl FromIterator<(String, PathBuf)> for MountTable {
    fn from_iter<T: IntoIterator<Item = (String, PathBuf)>>(iter: T) -> Self {
        Self {
            entries: iter
                .into_iter()
                .map(|(k, p)| (normalize_key(&k), p))
                .collect(),
        }
    }
}

pub fn parse_mount(s: &str) -> Result<(String, PathBuf), String> {
    let (host, path) = s
        .split_once('=')
        .ok_or_else(|| "mount must be HOST=PATH".to_string())?;
    if host.is_empty() {
        return Err("mount host must not be empty".to_string());
    }
    if path.is_empty() {
        return Err("mount path must not be empty".to_string());
    }
    Ok((normalize_key(host), PathBuf::from(path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn table(pairs: &[(&str, &str)]) -> MountTable {
        pairs
            .iter()
            .map(|(h, p)| (h.to_string(), PathBuf::from(p)))
            .collect()
    }

    #[test]
    fn get_exact_host_case_insensitive() {
        let t = table(&[("dsm", "/volume1/photo")]);
        assert_eq!(t.get("dsm"), Some(&PathBuf::from("/volume1/photo")));
        assert_eq!(t.get("DSM"), Some(&PathBuf::from("/volume1/photo")));
        assert_eq!(t.get("Dsm"), Some(&PathBuf::from("/volume1/photo")));
    }

    #[test]
    fn get_falls_back_to_star() {
        let t = table(&[("*", "/default")]);
        assert_eq!(t.get("unknown"), Some(&PathBuf::from("/default")));
    }

    #[test]
    fn get_prefers_exact_host_over_star() {
        let t = table(&[("dsm", "/photo"), ("*", "/default")]);
        assert_eq!(t.get("dsm"), Some(&PathBuf::from("/photo")));
        assert_eq!(t.get("other"), Some(&PathBuf::from("/default")));
    }

    #[test]
    fn get_none_when_no_match_and_no_star() {
        let t = table(&[("dsm", "/photo")]);
        assert_eq!(t.get("nope"), None);
    }

    #[test]
    fn star_key_not_lowercased_but_lookup_uses_star() {
        let t = table(&[("*", "/default")]);
        assert_eq!(t.get("*"), Some(&PathBuf::from("/default")));
    }

    #[test]
    fn parse_mount_ok_normalizes_host() {
        assert_eq!(
            parse_mount("DSM=/volume1/photo").unwrap(),
            ("dsm".to_string(), PathBuf::from("/volume1/photo"))
        );
        assert_eq!(
            parse_mount("*=/www").unwrap(),
            ("*".to_string(), PathBuf::from("/www"))
        );
    }

    #[test]
    fn parse_mount_rejects_bad_forms() {
        assert!(parse_mount("novalue").is_err());
        assert!(parse_mount("=path").is_err());
        assert!(parse_mount("host=").is_err());
    }
}
