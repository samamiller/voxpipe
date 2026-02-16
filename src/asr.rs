use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum AsrError {
    WhisperCliNotFound(String),
    ProcessFailed {
        code: Option<i32>,
        stderr: String,
        stdout: String,
    },
    EmptyTranscript {
        stderr: String,
    },
}

impl fmt::Display for AsrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WhisperCliNotFound(bin) => write!(
                f,
                "whisper-cli executable not found: {bin} (set VOXPIPE_WHISPER_CLI if needed)"
            ),
            Self::ProcessFailed {
                code,
                stderr,
                stdout,
            } => write!(
                f,
                "whisper-cli failed (code: {:?})\nstderr: {}\nstdout: {}",
                code,
                stderr.trim(),
                stdout.trim()
            ),
            Self::EmptyTranscript { stderr } => {
                write!(
                    f,
                    "whisper-cli produced no transcript\nstderr: {}",
                    stderr.trim()
                )
            }
        }
    }
}

impl std::error::Error for AsrError {}

pub fn transcribe(
    wav_path: impl AsRef<Path>,
    model_path: impl AsRef<Path>,
    args: &[&str],
) -> Result<String, AsrError> {
    let wav_path = wav_path.as_ref();
    let model_path = model_path.as_ref();
    let bin = whisper_cli_binary();

    let mut cmd = Command::new(&bin);
    cmd.arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(wav_path)
        .args(args);

    let output = cmd
        .output()
        .map_err(|_| AsrError::WhisperCliNotFound(bin))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(AsrError::ProcessFailed {
            code: output.status.code(),
            stderr,
            stdout,
        });
    }

    let transcript = extract_transcript(&stdout);
    if transcript.trim().is_empty() {
        return Err(AsrError::EmptyTranscript { stderr });
    }

    Ok(transcript)
}

pub fn whisper_cli_binary() -> String {
    std::env::var("VOXPIPE_WHISPER_CLI").unwrap_or_else(|_| "whisper-cli".to_string())
}

pub fn model_path_from_env_or_args() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("ASR_MODEL") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    let args: Vec<String> = std::env::args().collect();
    let mut idx = 0usize;
    while idx + 1 < args.len() {
        if args[idx] == "--model" {
            let value = args[idx + 1].trim();
            if !value.is_empty() {
                return Some(PathBuf::from(value));
            }
        }
        idx += 1;
    }

    None
}

fn extract_transcript(stdout: &str) -> String {
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("whisper_"))
        .collect();

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{transcribe, AsrError};
    use std::error::Error;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn write_cli_script(name: &str, body: &str) -> Result<std::path::PathBuf, Box<dyn Error>> {
        let dir = std::env::temp_dir().join("voxpipe-asr-tests");
        fs::create_dir_all(&dir)?;
        let path = dir.join(name);
        fs::write(&path, body)?;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms)?;
        Ok(path)
    }

    #[test]
    fn transcribe_captures_stdout() -> Result<(), Box<dyn Error>> {
        let script = write_cli_script(
            "fake-whisper-ok.sh",
            "#!/bin/sh\necho \"hello from asr\"\nexit 0\n",
        )?;
        // SAFETY: unit tests run in-process and intentionally control process env for this test.
        unsafe { std::env::set_var("VOXPIPE_WHISPER_CLI", &script) };

        let text = transcribe("/tmp/in.wav", "/tmp/model.bin", &[])?;
        if !text.contains("hello from asr") {
            return Err("expected transcript text in stdout".into());
        }
        Ok(())
    }

    #[test]
    fn transcribe_captures_stderr_on_failure() -> Result<(), Box<dyn Error>> {
        let script = write_cli_script(
            "fake-whisper-fail.sh",
            "#!/bin/sh\necho \"boom\" 1>&2\nexit 12\n",
        )?;
        // SAFETY: unit tests run in-process and intentionally control process env for this test.
        unsafe { std::env::set_var("VOXPIPE_WHISPER_CLI", &script) };

        let err = transcribe("/tmp/in.wav", "/tmp/model.bin", &[])
            .err()
            .ok_or("expected failure")?;
        match err {
            AsrError::ProcessFailed { code, stderr, .. } => {
                if code != Some(12) {
                    return Err("unexpected exit code".into());
                }
                if !stderr.contains("boom") {
                    return Err("expected stderr contents".into());
                }
            }
            _ => return Err("unexpected error type".into()),
        }
        Ok(())
    }
}
