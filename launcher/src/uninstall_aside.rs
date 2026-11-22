//! Where the Windows uninstaller puts its own running `.exe` (`installer::uninstall`).
//!
//! A running file can be renamed but not deleted, and only renamed within its volume: moving it
//! into a %TEMP% on another drive fails with ERROR_NOT_SAME_DEVICE (os error 17). So it goes to
//! %TEMP% when that is on the install folder's drive, else beside the install folder or inside
//! it when installing at the volume root. Plain path text, so it is tested on every OS.

/// The folders to try, in order, for the running launcher at `install\horizon-launcher.exe`.
/// %TEMP% comes first only when it is on the same drive. A fallback on the install volume
/// is needed even then because %TEMP% may be behind a junction.
pub fn aside_dirs(install: &str, temp: &str) -> Vec<String> {
    let mut dirs = Vec::new();
    let install_volume = volume(install);
    if install_volume.is_some() && install_volume == volume(temp) {
        dirs.push(temp.to_string());
    }
    if let Some(parent) = parent(install).filter(|p| volume(p) == install_volume) {
        dirs.push(parent);
    } else if install_volume.is_some() {
        dirs.push(install.to_string());
    }
    dirs
}

/// `C:` for `C:\...`, `\\server\share` for UNC paths (lowercase), also behind `\\?\`.
/// None for a relative path: its volume is unknown.
fn volume(path: &str) -> Option<String> {
    let path = path.replace('/', "\\");
    let path = if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(&path).to_string()
    };
    if let Some(rest) = path.strip_prefix(r"\\") {
        let mut parts = rest.split('\\').filter(|p| !p.is_empty());
        let (server, share) = (parts.next()?, parts.next()?);
        return Some(format!(r"\\{server}\{share}").to_lowercase());
    }
    let bytes = path.as_bytes();
    (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        .then(|| path[..2].to_ascii_uppercase())
}

/// The folder holding `path`, with Windows separators (either slash).
fn parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches(['\\', '/']);
    let cut = trimmed.rfind(['\\', '/'])?;
    let parent = &trimmed[..cut];
    // `C:\Horizon Launcher` → `C:\`, not the drive-relative `C:`.
    Some(if parent.ends_with(':') {
        format!(r"{parent}\")
    } else {
        parent.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_on_another_drive_puts_it_beside_the_install_folder() {
        // GitHub's Windows runner: the smoke installs under D:, TEMP is on C:.
        assert_eq!(
            aside_dirs(
                r"D:\a\_temp/setup-smoke\Horizon Launcher",
                r"C:\Users\RUNNER~1\AppData\Local\Temp"
            ),
            vec![r"D:\a\_temp/setup-smoke".to_string()]
        );
        assert_eq!(
            aside_dirs(r"\\nas\games\Horizon Launcher", r"C:\Temp"),
            vec![r"\\nas\games".to_string()]
        );
    }

    #[test]
    fn a_volume_root_holds_its_own_uninstall_copy() {
        for root in [r"D:\", r"\\?\D:\", r"\\nas\games", r"\\?\UNC\nas\games\"] {
            assert_eq!(aside_dirs(root, r"C:\Temp"), vec![root]);
        }
        assert_eq!(aside_dirs(r"D:\", r"D:\Temp"), vec![r"D:\Temp", r"D:\"]);
    }

    #[test]
    fn temp_on_the_same_drive_comes_first() {
        assert_eq!(
            aside_dirs(
                r"C:\Users\p\AppData\Local\Programs\Horizon Launcher",
                r"c:\Users\p\AppData\Local\Temp\"
            ),
            vec![
                r"c:\Users\p\AppData\Local\Temp\".to_string(),
                r"C:\Users\p\AppData\Local\Programs".to_string()
            ]
        );
        assert_eq!(
            aside_dirs(r"\\?\D:\Horizon Launcher\", r"D:\Temp"),
            vec![r"D:\Temp".to_string(), r"\\?\D:\".to_string()]
        );
        assert_eq!(
            aside_dirs(r"\\NAS\Games\Horizon Launcher", r"\\?\UNC\nas\games\tmp"),
            vec![
                r"\\?\UNC\nas\games\tmp".to_string(),
                r"\\NAS\Games".to_string()
            ]
        );
    }

    #[test]
    fn an_unknown_temp_volume_is_never_trusted() {
        assert_eq!(
            aside_dirs(r"C:\Games\Horizon Launcher", "tmp"),
            vec![r"C:\Games".to_string()]
        );
    }
}
