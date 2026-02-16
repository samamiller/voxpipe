use std::ffi::{CStr, CString};
use std::fmt;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

use crate::whisper_sys;

#[derive(Debug)]
pub enum WhisperError {
    ModelPathNotUtf8(PathBuf),
    ModelPathContainsNul(PathBuf),
    ContextInitFailed(PathBuf),
}

impl fmt::Display for WhisperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelPathNotUtf8(path) => write!(f, "model path is not valid UTF-8: {}", path.display()),
            Self::ModelPathContainsNul(path) => {
                write!(f, "model path contains interior NUL bytes: {}", path.display())
            }
            Self::ContextInitFailed(path) => write!(f, "failed to initialize whisper context: {}", path.display()),
        }
    }
}

impl std::error::Error for WhisperError {}

pub fn system_info() -> String {
    // SAFETY: whisper.cpp returns a process-global, NUL-terminated string pointer.
    // We only read it and convert into an owned Rust String.
    let ptr = unsafe { whisper_sys::whisper_print_system_info() };
    if ptr.is_null() {
        return "unavailable".to_string();
    }

    // SAFETY: pointer is expected to reference a valid C string for the process lifetime.
    unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
}

pub struct WhisperContext {
    raw: NonNull<whisper_sys::whisper_context>,
}

impl WhisperContext {
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self, WhisperError> {
        let model_path = model_path.as_ref();
        let model_path_buf = model_path.to_path_buf();
        let model_path_str = model_path
            .to_str()
            .ok_or_else(|| WhisperError::ModelPathNotUtf8(model_path_buf.clone()))?;
        let c_model_path = CString::new(model_path_str.as_bytes())
            .map_err(|_| WhisperError::ModelPathContainsNul(model_path_buf.clone()))?;

        // SAFETY: c_model_path is a valid NUL-terminated C string for the duration of the call.
        // whisper_init_from_file returns either a valid non-null context pointer or null on failure.
        let raw = unsafe { whisper_sys::whisper_init_from_file(c_model_path.as_ptr()) };
        let raw = NonNull::new(raw).ok_or(WhisperError::ContextInitFailed(model_path_buf))?;

        Ok(Self { raw })
    }
}

impl Drop for WhisperContext {
    fn drop(&mut self) {
        // SAFETY: self.raw is created from whisper_init_from_file and owned by this wrapper.
        unsafe { whisper_sys::whisper_free(self.raw.as_ptr()) };
    }
}
