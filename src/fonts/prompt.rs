use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long a prompt capture may run before it is killed.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(3);

/// Capture up to two lines of the shell prompt, never blocking the caller for
/// longer than [`PROMPT_TIMEOUT`].
///
/// Uses `shell` when given, otherwise `$SHELL`, otherwise `sh`. `zsh` uses
/// `print -P "$PROMPT"`, `bash` uses `${PS1@P}`; anything else returns the
/// `$ ` fallback without spawning a child.
pub fn capture_prompt_lines(shell: Option<&str>) -> Vec<String> {
    let shell = shell
        .map(str::to_owned)
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "sh".to_owned());
    let command = if shell.ends_with("zsh") {
        "print -P \"$PROMPT\""
    } else if shell.ends_with("bash") {
        "printf '%s' \"${PS1@P}\""
    } else {
        return vec!["$ ".to_owned()];
    };

    let mut child = match Command::new(&shell)
        .args(["-ic", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return vec!["$ ".to_owned()],
    };

    let start = Instant::now();
    loop {
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        if start.elapsed() >= PROMPT_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return vec!["$ ".to_owned()];
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut output);
    }
    let lines: Vec<String> = output
        .lines()
        .take(2)
        .map(str::to_owned)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        vec!["$ ".to_owned()]
    } else {
        lines
    }
}
