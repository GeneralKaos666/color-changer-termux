use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

pub fn capture_prompt_lines() -> Vec<String> {
    // Try zsh
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg("SHELL=zsh $SHELL -ic 'print -P \"$PROMPT\"' 2>/dev/null")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return vec!["$ ".to_string()],
    };
    let _ = std::thread::sleep(Duration::from_millis(500));
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut output);
    }
    let _ = child.kill();
    let lines: Vec<String> = output
        .lines()
        .take(2)
        .map(|l| l.to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() { vec!["$ ".to_string()] } else { lines }
}
