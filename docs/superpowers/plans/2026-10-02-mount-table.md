# Mount Table (`--mount`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a repeatable clap `--mount HOST=PATH` option to `album` and a `MountTable` type with hostname lookup (`host` → `*` → `None`), keys case-insensitive.

**Architecture:** New module `album/src/mount.rs` owns `MountTable` (entries stored with lowercase keys; special key `*`) and `parse_mount` for clap. `main.rs` gains a `mount: Vec<(String, PathBuf)>` clap field, collects into `MountTable` after parse, logs mounts at startup. Lookup is a member function; no HTTP consumer in this plan (method is the deliverable).

**Tech Stack:** Rust, clap 4 (derive, already in tree), existing `album` crate.

**Spec:** In-chat design approved this session — binding decisions:
- CLI name `--mount`, form `HOST=PATH`, repeatable; `value_name = HOST=PATH`
- Special key `*` = default entry (shell requires quoting: `--mount '*=...'`)
- Hostname keys **case-insensitive via ASCII lowercase** on insert and lookup
- Lookup order: normalized hostname → `*` → `None` (return `Option<&PathBuf>`)
- Empty path (`HOST=`) is a **parse error** (not silent default)
- Member function on `MountTable`, not free function
- Spike reference (throwaway, do not ship): `/tmp/opencode/clap-kv-probe`

## Global Constraints

- Backend crate root: `/workspace/album/`; do not modify `ui/`.
- Existing clap fields `--port` (default 3000) and `--addr` (default `0.0.0.0`) keep working; `PORT` env stays deprecated.
- Logging: `tracing` → stdout at fixed `info` (no `--log-level`, no `RUST_LOG`).
- Special default key is exactly `*` (not empty string).
- `get` returns `Option<&PathBuf>`; order host → `*` → `None`.
- Never stage or read `/workspace/.env`. Commit only intended paths under `album/` (+ README if CLI help text must be documented).
- After each task: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, full `cargo test` green.

## Review Focus

- **Case folding:** `DSM` / `dsm` / `Dsm` must hit the same entry — pin with unit test asserting `get("DSM")` == `get("dsm")` when only `dsm=...` was inserted (Task 1).
- **Fallback order:** host miss must try `*`, not return `None` while `*` exists; host hit must NOT fall through to `*` — pin with two unit tests (Task 1).
- **Empty path rejected:** `--mount host=` fails parse with clear error — pin with `try_parse_from` test (Task 2).
- **Existing CLI intact:** `--port`/`--addr` defaults unchanged after adding `--mount` — pin with parse-defaults test (Task 2).
- **Glob quoting:** docs/help must show `'*=...'` form so shells do not expand `*` — pin in README snippet (Task 2).

---

### Task 1: `MountTable` type + `parse_mount` (TDD)

**Files:**
- Create: `/workspace/album/src/mount.rs`
- Modify: `/workspace/album/src/lib.rs` (`pub mod mount;`)
- Test: unit tests in `mount.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: nothing
- Produces:
  - `pub struct MountTable` (field private)
  - `impl MountTable { pub fn get(&self, hostname: &str) -> Option<&PathBuf> }`
  - `impl FromIterator<(String, PathBuf)> for MountTable` (lowercases non-`*` keys)
  - `pub fn parse_mount(s: &str) -> Result<(String, PathBuf), String>` — clap-compatible; rejects missing `=`, empty key, empty path; non-`*` keys lowercased; `*` preserved

- [ ] **Step 1: Write failing unit tests in `mount.rs`**

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /workspace/album && cargo test --lib mount`
Expected: FAIL (module / functions not defined)

- [ ] **Step 3: Implement `mount.rs` + wire `pub mod mount;` in `lib.rs`**

- `MountTable { entries: Vec<(String, PathBuf)> }` private field
- `get`: `key = hostname.to_ascii_lowercase()` → find `entries` where `k == key` → else find `k == "*"` → map to path
- `FromIterator`: lowercase each key unless key is `*`
- `parse_mount`: `split_once('=')` errors if absent; empty key or empty path → `Err`; normalize like FromIterator

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /workspace/album && cargo test --lib mount`
Expected: PASS (all 7)

- [ ] **Step 5: fmt / clippy / full test**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: all green

- [ ] **Step 6: Commit**

```bash
git -C /workspace add album/src/mount.rs album/src/lib.rs
git -C /workspace commit -m "feat(album): MountTable with case-insensitive host and * fallback"
```

---

### Task 2: clap `--mount` + startup collect (TDD)

**Files:**
- Modify: `/workspace/album/src/main.rs` (`Args` + `main`)
- Modify: `/workspace/README.md` (CLI bullet: `--mount` example with quoted `*`)
- Test: unit tests in `main.rs` or `album/tests/cli_mount.rs` using `Args::try_parse_from` (export `Args` as `pub(crate)` or test via `clap::CommandFactory` — prefer `pub struct Args` in `main.rs` only if binary tests need it; otherwise put parse tests in `mount.rs` against `parse_mount` already done, and add binary-level test only for defaults if `Args` is reachable)

**Interfaces:**
- Consumes: `album::mount::{parse_mount, MountTable}` from Task 1
- Produces:
  - `Args` field: `mount: Vec<(String, PathBuf)>` with `#[arg(long = "mount", value_name = "HOST=PATH", value_parser = parse_mount)]`
  - `main`: `let mounts: MountTable = args.mount.into_iter().collect();` then `tracing::info!("mounts: ...")` (debug-print each entry; empty table logs `mounts: (none)`)
  - README documents `--mount` and quoted `*`

- [ ] **Step 1: Write failing test for CLI field shape**

Prefer testing parse without spawning the server: in `album/src/main.rs` under `#[cfg(test)]`, or if awkward for bin targets, add `album/tests` only if `Args` is moved/exposed — **implementer choice that keeps one clear test**: simplest path is `#[cfg(test)] mod tests` in `main.rs` using `Args::try_parse_from(["album", "--mount", "DSM=/x", "--mount", "*=/y"])` asserting `args.mount == vec![("dsm".into(), "/x".into()), ("*".into(), "/y".into())]` **only if** `parse_mount` already lowercases (assert through parser results). Also assert defaults: `try_parse_from(["album"])` → port 3000, addr 0.0.0.0, mount empty.

If bin unit tests are awkward, move `Args` to `lib.rs` as `pub struct CliArgs` and have `main` call it — Interfaces above still hold; record choice in report.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /workspace/album && cargo test`
Expected: FAIL (no `mount` field yet)

- [ ] **Step 3: Implement clap field + `main` collect + log**

Exact field attributes as in Interfaces. After `Args::parse()`:

```rust
let mounts: MountTable = args.mount.into_iter().collect();
// tracing::info! each (host, path) or "mounts: (none)"
```

Do **not** change `--port`/`--addr`. Do not add `--log-level`.

- [ ] **Step 4: README**

Under Development CLI bullet, add:

```
--mount HOST=PATH   repeatable; hostname case-insensitive; '*' is default (quote it: --mount '*=/path')
```

Example: `cargo run -- --mount 'dsm=/volume1/photo' --mount '*=/workspace/.www'`

- [ ] **Step 5: Run full test + help smoke**

Run: `cargo test && cargo run -q -- --help`
Expected: tests green; help shows `--mount <HOST=PATH>`; defaults for port/addr unchanged.

- [ ] **Step 6: Commit**

```bash
git -C /workspace add album README.md
git -C /workspace commit -m "feat(album): --mount HOST=PATH collected into MountTable"
```

---

## Self-review notes (writer)

- Spec coverage: all approved design bullets map to Task 1 (type/lookup/parse) or Task 2 (CLI/README/log).
- Empty path rejected in `parse_mount` tests; `*` fallback order covered; case fold covered.
- No HTTP consumer — intentional per YAGNI; `get` is unit-tested deliverable.