use std::path::Path;

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
}
