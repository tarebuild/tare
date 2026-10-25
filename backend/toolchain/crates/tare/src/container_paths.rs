use anyhow::{bail, Result};

#[derive(Debug, PartialEq)]
pub struct ContainerPaths {
    pub workdir: String,
    pub manifest: String,
}

// Container paths are POSIX strings, even when the caller runs on Windows.
pub fn map(root: &str, directory: &str, manifest: &str) -> Result<ContainerPaths> {
    fn normalize(path: &str) -> Result<String> {
        let path = path.strip_prefix(r"\\?\").unwrap_or(path).replace('\\', "/");
        if !(path.starts_with('/') || path.as_bytes().get(1) == Some(&b':')) {
            bail!("container mapping requires an absolute host path: {path}");
        }
        let mut parts = Vec::new();
        for part in path.split('/') {
            match part {
                "" | "." => {},
                ".." => { if parts.pop().is_none() { bail!("path escapes its root") } },
                _ => parts.push(part),
            }
        }
        Ok(format!("{}{}", if path.starts_with('/') { "/" } else { "" }, parts.join("/")))
    }
    let root = normalize(root)?;
    let windows = root.as_bytes().get(1) == Some(&b':');
    let inside = |path: &str| -> Result<String> {
        let path = normalize(path)?;
        let equal = |a: &str, b: &str| if windows { a.eq_ignore_ascii_case(b) } else { a == b };
        if equal(&path, &root) { return Ok("/workspace".into()) }
        if path.len() <= root.len() || path.get(..root.len()).is_none_or(|prefix|!equal(prefix, &root)) || path.as_bytes()[root.len()] != b'/' {
            bail!("path is outside the mounted source root: {path}");
        }
        Ok(format!("/workspace{}", &path[root.len()..]))
    };
    Ok(ContainerPaths { workdir: inside(directory)?, manifest: inside(manifest)? })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_drive_and_nested_manifest() {
        assert_eq!(map(r"C:\src\anchor", r"C:\src\anchor\tests\idl", r"C:\src\anchor\tests\idl\programs\new-idl\Cargo.toml").unwrap(), ContainerPaths { workdir: "/workspace/tests/idl".into(), manifest: "/workspace/tests/idl/programs/new-idl/Cargo.toml".into() });
    }
    #[test]
    fn windows_extended_drive_case_and_spaces() {
        assert_eq!(map(r"\\?\D:\My Work\anchor", r"d:\My Work\anchor\tests\..\examples", r"D:/My Work/anchor/examples/basic/Cargo.toml").unwrap().manifest, "/workspace/examples/basic/Cargo.toml");
    }
    #[test]
    fn posix_workspace_above_current_folder() {
        let mapped = map("/src/anchor", "/src/anchor/tests/idl/programs/new-idl", "/src/anchor/tests/idl/programs/new-idl/Cargo.toml").unwrap();
        assert_eq!(mapped.workdir, "/workspace/tests/idl/programs/new-idl");
        assert!(!mapped.manifest.contains('\\'));
    }
    #[test]
    fn unicode_paths_keep_component_boundaries() {
        assert_eq!(map("/src/\u{D55C}\u{AE00} workspace", "/src/\u{D55C}\u{AE00} workspace/\u{D504}\u{B85C}", "/src/\u{D55C}\u{AE00} workspace/\u{D504}\u{B85C}/Cargo.toml").unwrap(), ContainerPaths {
            workdir: "/workspace/\u{D504}\u{B85C}".into(), manifest: "/workspace/\u{D504}\u{B85C}/Cargo.toml".into(),
        });
        assert!(map("/src/\u{D55C}\u{AE00}", "/src/\u{D55C}\u{AE00}-extra", "/src/\u{D55C}\u{AE00}/Cargo.toml").is_err());
    }
    #[test]
    fn root_manifest_and_escape_rejected() {
        assert_eq!(map("/src/a", "/src/a", "/src/a/Cargo.toml").unwrap().workdir, "/workspace");
        assert!(map("/src/a", "/src/ab", "/src/a/Cargo.toml").is_err());
        assert!(map("/src/a", "/src/a", "/src/a/../b/Cargo.toml").is_err());
        assert!(map(r"C:\a", r"D:\a", r"C:\a\Cargo.toml").is_err());
    }
}
