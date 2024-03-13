use std::io;
use std::fs::Metadata;
use std::{path::Path, fs::File};

#[derive(Debug)]
pub struct FileIO<S: AsRef<Path>> {
    pub filename: S,
    pub options: [bool; 3],
    buf_size: usize,
}

impl<S: AsRef<Path>> FileIO<S> {
    pub fn new(filename: S, options: [bool; 3]) -> Self {
        Self {
            filename,
            options,
            buf_size: 8192usize, // Default capacity for BufRead/Writer
        }
    }
    pub fn with_buf_size<N: Into<usize>>(&mut self, buf_size: N) {
        self.buf_size = buf_size.into();
    }

    pub fn get_buf_size(&self) -> usize {
        self.buf_size
    }

    pub fn set_buf_size<N: Into<usize>>(&mut self, buf_size: N) {
        self.with_buf_size(buf_size);
    }

    /// # Errors
    /// See [`std::fs::File::metadata`].
    pub fn metadata(&self, f: &File) -> io::Result<Metadata> {
        f.metadata()
    }
    pub fn exists(&self, f: &File) -> bool {
        self.metadata(f).is_ok()
    }
}
