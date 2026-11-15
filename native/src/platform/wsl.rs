// Claude Code run inside WSL keeps its logs in the distro's own home, which
// Windows reaches as \\wsl.localhost\<distro>\home\<user>\.claude. Only
// running distros are looked at: opening a stopped one's share would boot it.

use std::path::PathBuf;

/// A Claude config dir inside a distro, with the distro's `/` as Windows sees
/// it (project names are paths from there).
#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeDir {
    pub root: PathBuf,
    pub dir: PathBuf,
}

/// The Claude config dirs of every running distro that has any logs.
pub fn claude_dirs() -> Vec<ClaudeDir> {
    let mut out = Vec::new();
    for distro in running() {
        let Some(root) = [r"\\wsl.localhost", r"\\wsl$"].iter().map(|share| PathBuf::from(format!(r"{share}\{distro}\"))).find(|r| r.join("home").is_dir())
        else {
            continue;
        };
        let mut homes: Vec<PathBuf> = std::fs::read_dir(root.join("home")).map(|it| it.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        homes.sort();
        homes.push(root.join("root"));
        for home in homes {
            let dir = home.join(".claude");
            if dir.join("projects").is_dir() {
                out.push(ClaudeDir { root: root.clone(), dir });
            }
        }
    }
    out
}

/// `wsl.exe --list --running --quiet`, without a console flashing up, and
/// never stuck on a WSL that doesn't answer.
#[cfg(windows)]
fn running() -> Vec<String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let Ok(mut child) = Command::new("wsl.exe")
        .args(["--list", "--running", "--quiet"])
        .env("WSL_UTF8", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    else {
        return vec![];
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < Duration::from_secs(3) => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return vec![];
            }
        }
    }
    let mut bytes = Vec::new();
    let _ = child.stdout.take().map(|mut s| s.read_to_end(&mut bytes));
    distros(&bytes)
}

#[cfg(not(windows))]
fn running() -> Vec<String> {
    vec![]
}

/// The distro names in wsl.exe's list: UTF-8 when it honours WSL_UTF8, else
/// UTF-16LE. Anything that can't be a distro name (a message, a blank line)
/// is dropped.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn distros(bytes: &[u8]) -> Vec<String> {
    // UTF-16LE: most characters are ASCII, so most high bytes are zero
    let zero_high = bytes.iter().skip(1).step_by(2).filter(|&&b| b == 0).count();
    let text = if !bytes.is_empty() && bytes.len().is_multiple_of(2) && zero_high * 2 >= bytes.len() / 2 {
        let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    text.lines()
        .map(|l| l.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}' || c == '\0'))
        .filter(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::distros;

    #[test]
    fn reads_utf8_and_utf16_lists() {
        assert_eq!(distros(b"Ubuntu\r\nDebian\r\n"), vec!["Ubuntu", "Debian"]);
        let utf16: Vec<u8> = "\u{feff}Ubuntu-24.04\r\n".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(distros(&utf16), vec!["Ubuntu-24.04"]);
    }

    #[test]
    fn drops_messages_and_blank_lines() {
        assert!(distros(b"There are no running distributions.\r\n\r\n").is_empty());
        assert!(distros(b"").is_empty());
    }
}
