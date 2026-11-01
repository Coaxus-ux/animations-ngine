use std::io::Write as _;
use std::process::{Command, Stdio};

#[cfg(target_os = "macos")]
const WRITERS: &[&[&str]] = &[&["pbcopy"]];
#[cfg(target_os = "macos")]
const READERS: &[&[&str]] = &[&["pbpaste"]];

#[cfg(target_os = "windows")]
const WRITERS: &[&[&str]] = &[&["clip"]];
#[cfg(target_os = "windows")]
const READERS: &[&[&str]] = &[&["powershell", "-NoProfile", "-Command", "Get-Clipboard"]];

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const WRITERS: &[&[&str]] = &[&["wl-copy"], &["xclip", "-selection", "clipboard"]];
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const READERS: &[&[&str]] = &[
    &["wl-paste", "--no-newline"],
    &["xclip", "-selection", "clipboard", "-o"],
];

#[derive(Default)]
pub(super) struct Clipboard {
    fallback: Option<String>,
}

impl Clipboard {
    pub(super) fn write(&mut self, text: String) {
        if !WRITERS.iter().any(|command| pipe_into(command, &text)) {
            self.fallback = Some(text);
        }
    }

    pub(super) fn read(&self) -> Option<String> {
        READERS
            .iter()
            .find_map(|command| read_from(command))
            .or_else(|| self.fallback.clone())
    }
}

fn pipe_into(command: &[&str], text: &str) -> bool {
    let Some((program, args)) = command.split_first() else {
        return false;
    };
    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let written = child
        .stdin
        .take()
        .is_some_and(|mut stdin| stdin.write_all(text.as_bytes()).is_ok());
    child.wait().is_ok_and(|status| status.success()) && written
}

fn read_from(command: &[&str]) -> Option<String> {
    let (program, args) = command.split_first()?;
    let output = Command::new(program)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}
