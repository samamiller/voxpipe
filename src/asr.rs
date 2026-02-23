use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_MODEL_NAME: &str = "ggml-base.en-q5_1.bin";
const DEFAULT_FETCH_NAME: &str = "base.en-q5_1";

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
    JsonParse {
        message: String,
    },
    ModelMissing {
        message: String,
        fix_command: String,
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
            Self::JsonParse { message } => write!(f, "whisper-cli JSON parse failed: {message}"),
            Self::ModelMissing {
                message,
                fix_command,
            } => write!(f, "{message}\nFix: {fix_command}"),
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
    let cli = resolve_whisper_cli();
    let bin = cli.binary.to_string_lossy().into_owned();

    let mut cmd = Command::new(&cli.binary);
    if let Some(lib_dir) = cli.lib_dir {
        let merged = merge_library_path(&lib_dir);
        cmd.env("LD_LIBRARY_PATH", merged);
    }
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

#[derive(Clone, Debug)]
pub struct ConfidenceSegment {
    pub text: String,
    pub tokens: Vec<(String, f32)>,
}

pub type ConfidenceSegments = Vec<ConfidenceSegment>;

pub fn transcribe_with_confidence(
    wav_path: impl AsRef<Path>,
    model_path: impl AsRef<Path>,
) -> Result<ConfidenceSegments, AsrError> {
    let wav_path = wav_path.as_ref();
    let model_path = model_path.as_ref();
    let cli = resolve_whisper_cli();
    let bin = cli.binary.to_string_lossy().into_owned();

    let mut cmd = Command::new(&cli.binary);
    if let Some(lib_dir) = cli.lib_dir {
        let merged = merge_library_path(&lib_dir);
        cmd.env("LD_LIBRARY_PATH", merged);
    }
    cmd.arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(wav_path)
        .args([
            "-nt",
            "--print-confidence",
            "--output-json-full",
            "--output-file",
            "-",
            "--no-prints",
        ]);

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

    let parsed: JsonTranscript = serde_json::from_str(stdout.trim())
        .map_err(|err| AsrError::JsonParse { message: err.to_string() })?;
    let segments = parsed
        .transcription
        .into_iter()
        .map(|segment| ConfidenceSegment {
            text: segment.text,
            tokens: segment
                .tokens
                .into_iter()
                .map(|token| (token.text, token.p))
                .collect(),
        })
        .collect();

    Ok(segments)
}

#[derive(Deserialize)]
struct JsonTranscript {
    transcription: Vec<JsonSegment>,
}

#[derive(Deserialize)]
struct JsonSegment {
    text: String,
    #[serde(default)]
    tokens: Vec<JsonToken>,
}

#[derive(Deserialize)]
struct JsonToken {
    text: String,
    p: f32,
}

struct WhisperCliLocation {
    binary: PathBuf,
    lib_dir: Option<PathBuf>,
}

fn resolve_whisper_cli() -> WhisperCliLocation {
    if let Ok(bin) = std::env::var("VOXPIPE_WHISPER_CLI") {
        let trimmed = bin.trim();
        if !trimmed.is_empty() {
            return WhisperCliLocation {
                binary: PathBuf::from(trimmed),
                lib_dir: None,
            };
        }
    }

    if let Some(local) = discover_local_whisper_cli() {
        return local;
    }

    WhisperCliLocation {
        binary: PathBuf::from("whisper-cli"),
        lib_dir: None,
    }
}

fn discover_local_whisper_cli() -> Option<WhisperCliLocation> {
    let cwd = std::env::current_dir().ok()?;
    let mut candidates: Vec<(PathBuf, PathBuf, std::time::SystemTime)> = Vec::new();

    for profile in ["debug", "release"] {
        let build_root = cwd.join("target").join(profile).join("build");
        let entries = match fs::read_dir(build_root) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let out_dir = entry.path().join("out");
            let bin = out_dir.join("bin").join("whisper-cli");
            if !bin.is_file() {
                continue;
            }
            let lib_dir = out_dir.join("lib");
            let modified = fs::metadata(&bin)
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            candidates.push((bin, lib_dir, modified));
        }
    }

    candidates.sort_by_key(|(_, _, modified)| std::cmp::Reverse(*modified));
    candidates.first().map(|(bin, lib_dir, _)| WhisperCliLocation {
        binary: bin.clone(),
        lib_dir: Some(lib_dir.clone()),
    })
}

fn merge_library_path(prefix: &Path) -> String {
    let prefix_text = prefix.to_string_lossy().into_owned();
    match std::env::var("LD_LIBRARY_PATH") {
        Ok(existing) if !existing.trim().is_empty() => format!("{prefix_text}:{existing}"),
        _ => prefix_text,
    }
}

pub fn discover_model_path() -> Result<PathBuf, AsrError> {
    let args: Vec<String> = std::env::args().collect();
    let cli_model = parse_model_from_args(&args);
    if let Some(path) = cli_model {
        return ensure_model_exists(path, "CLI --model");
    }

    if let Ok(path) = std::env::var("ASR_MODEL") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return ensure_model_exists(PathBuf::from(trimmed), "ASR_MODEL");
        }
    }

    if let Some(path) = discover_from_directory(&bundled_models_dir()) {
        return Ok(path);
    }

    if let Some(path) = discover_from_directory(&repo_models_dir()) {
        return Ok(path);
    }

    if let Some(path) = discover_from_directory(&cache_models_dir()) {
        return Ok(path);
    }

    Err(AsrError::ModelMissing {
        message: format!(
            "No Whisper model found via --model, ASR_MODEL, {}, {}, or {}",
            bundled_models_dir().display(),
            repo_models_dir().display(),
            cache_models_dir().display()
        ),
        fix_command: default_fix_command(),
    })
}

pub fn model_search_dirs() -> Vec<PathBuf> {
    vec![bundled_models_dir(), repo_models_dir(), cache_models_dir()]
}

pub fn model_default_dir() -> Option<PathBuf> {
    model_search_dirs()
        .into_iter()
        .find(|dir| dir.is_dir())
}

pub fn ffmpeg_enabled() -> bool {
    matches!(option_env!("VOXPIPE_WHISPER_FFMPEG"), Some("1"))
}

fn ensure_model_exists(path: PathBuf, source: &str) -> Result<PathBuf, AsrError> {
    if path.is_file() {
        return Ok(path);
    }

    Err(AsrError::ModelMissing {
        message: format!("{source} points to missing model file: {}", path.display()),
        fix_command: default_fix_command(),
    })
}

fn parse_model_from_args(args: &[String]) -> Option<PathBuf> {
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

fn discover_from_directory(dir: &Path) -> Option<PathBuf> {
    let preferred = dir.join(DEFAULT_MODEL_NAME);
    if preferred.is_file() {
        return Some(preferred);
    }

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("ggml-") && name.ends_with(".bin"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

pub fn cache_models_dir() -> PathBuf {
    if let Ok(path) = std::env::var("VOXPIPE_MODELS_CACHE_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cache/voxpipe/models")
}

fn bundled_models_dir() -> PathBuf {
    if let Ok(path) = std::env::var("VOXPIPE_MODELS_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    PathBuf::from("/usr/share/voxpipe/models")
}

fn repo_models_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("models")
}

fn default_fix_command() -> String {
    format!(
        "scripts/fetch-model.sh {DEFAULT_FETCH_NAME} && export ASR_MODEL=\"{}/{}\"",
        cache_models_dir().display(),
        DEFAULT_MODEL_NAME
    )
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
    use super::{AsrError, discover_from_directory, transcribe};
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

    #[test]
    fn discover_prefers_default_model_name() -> Result<(), Box<dyn Error>> {
        let dir = std::env::temp_dir().join("voxpipe-asr-model-discovery");
        fs::create_dir_all(&dir)?;
        let other = dir.join("ggml-base.en.bin");
        let default = dir.join("ggml-base.en-q5_1.bin");
        fs::write(&other, "x")?;
        fs::write(&default, "x")?;

        let path = discover_from_directory(&dir).ok_or("expected model path")?;
        if path != default {
            return Err("expected default tiny model preference".into());
        }
        Ok(())
    }
}
