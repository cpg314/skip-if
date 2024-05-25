use std::path::{Path, PathBuf};

use tracing::*;

use crate::Strategy;

pub struct FileExists;

impl<O> Strategy<O> for FileExists {
    fn skip(&self, output: &Path, _args_hash: u64, _code_hash: u64) -> bool {
        let exists = output.exists();
        if exists {
            warn!(?output, "Skipping as output exists");
        }
        exists
    }
    fn callback(
        &self,
        _fn_output: &O,
        _output: &Path,
        _args_hash: u64,
        _code_hash: u64,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

// See https://internals.rust-lang.org/t/14187
pub fn append_ext(ext: impl AsRef<std::ffi::OsStr>, path: &Path) -> PathBuf {
    let mut os_string: std::ffi::OsString = path.into();
    os_string.push(".");
    os_string.push(ext.as_ref());
    os_string.into()
}
pub struct Markers<E> {
    /// Use failure markers {output}.failure to skip previously failed calls.
    pub failure_marker: bool,
    pub retriable: Box<dyn Fn(&E) -> bool>,
    /// Use success markers {output}.success to rerun when arguments or code have changed.
    /// This only makes sense with the `hashes` flag.
    pub success_marker: bool,
    /// Use code and arguments hashes for markers. If they changed, rerun failed and successful runs.
    pub hashes: bool,
    /// Assume the `output` attribute is a folder and store the markers in {output}/success and
    /// {output}/failure.
    pub folder: bool,
}
impl<E> Default for Markers<E> {
    fn default() -> Self {
        Self {
            failure_marker: true,
            retriable: Box::new(|_| true),
            success_marker: true,
            hashes: true,
            folder: false,
        }
    }
}

impl<E> Markers<E> {
    fn marker_path(&self, success: bool, output: &Path) -> PathBuf {
        let name = if success { "success" } else { "failure" };
        if self.folder {
            output.join(name)
        } else {
            append_ext(name, output)
        }
    }
    fn hashes_str(&self, args_hash: u64, code_hash: u64) -> String {
        if self.hashes {
            format!("{}\n{}", args_hash, code_hash)
        } else {
            Default::default()
        }
    }
    pub fn folder(mut self) -> Self {
        self.folder = true;
        self
    }
    pub fn retriable(mut self, retriable: impl Fn(&E) -> bool + 'static) -> Self {
        self.retriable = Box::new(retriable);
        self
    }
}

impl<T, E> Strategy<Result<T, E>> for Markers<E> {
    fn skip(&self, output: &Path, args_hash: u64, code_hash: u64) -> bool {
        let check_marker = |path: &Path| {
            if let Ok(s) = std::fs::read_to_string(path) {
                if s == self.hashes_str(args_hash, code_hash) {
                    return true;
                }
            }
            false
        };
        if self.failure_marker {
            let marker = self.marker_path(false, output);
            // Failure skipping
            if check_marker(&marker) {
                warn!(?marker, "Skipping due to failure marker");
                return true;
            }
        }
        if self.success_marker {
            let marker = self.marker_path(true, output);
            match (check_marker(&marker), output.exists()) {
                (true, false) => {
                    warn!(?marker, "Success marker exists, but not the output file");
                    false
                }
                (true, true) => {
                    warn!(?marker, "Skipping due to success marker");
                    true
                }
                (false, _) => false,
            }
        } else {
            // Otherwise we rely on the file existence
            output.exists()
        }
    }
    fn callback(
        &self,
        result: &Result<T, E>,
        output: &Path,
        args_hash: u64,
        code_hash: u64,
    ) -> anyhow::Result<()> {
        let write_markers = |success: bool| {
            // Write the failure or success marker
            if self.folder {
                std::fs::create_dir_all(output)?;
            }
            let path = self.marker_path(success, output);
            debug!(
                ?path,
                "Writing {} marker",
                if success { "success" } else { "failure" }
            );
            std::fs::write(path, self.hashes_str(args_hash, code_hash))?;
            // Delete the other marker if it exists
            let _ = std::fs::remove_file(self.marker_path(!success, output));
            anyhow::Ok(())
        };
        match result {
            Ok(_) if self.success_marker => {
                write_markers(true)?;
            }
            Err(e) if self.failure_marker => {
                if (self.retriable)(e) {
                    debug!("Not writing a failure marker as the error is retriable");
                } else {
                    write_markers(false)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
