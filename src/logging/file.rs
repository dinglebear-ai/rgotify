use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::Mutex,
};

/// A simple file writer that caps at `max_bytes` by truncating the file when
/// the limit is reached. Exactly one log file is ever used.
pub struct CappedFileWriter {
    path: PathBuf,
    max_bytes: u64,
    file: Mutex<File>,
    current_size: Mutex<u64>,
}

impl CappedFileWriter {
    pub fn open(path: PathBuf, max_bytes: u64) -> io::Result<Self> {
        std::fs::create_dir_all(path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "log path has no parent")
        })?)?;

        // Truncate on open if already over limit
        let current_size = if path.exists() {
            let meta = std::fs::metadata(&path)?;
            if meta.len() >= max_bytes {
                // truncate
                std::fs::write(&path, b"")?;
                0
            } else {
                meta.len()
            }
        } else {
            0
        };

        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        Ok(Self {
            path,
            max_bytes,
            file: Mutex::new(file),
            current_size: Mutex::new(current_size),
        })
    }
}

impl Write for CappedFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_all_impl(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Ok(mut f) = self.file.lock() {
            f.flush()
        } else {
            Ok(())
        }
    }
}

impl CappedFileWriter {
    fn write_all_impl(&self, buf: &[u8]) -> io::Result<()> {
        let mut size = self.current_size.lock().unwrap();
        let mut file = self.file.lock().unwrap();

        *size += buf.len() as u64;
        if *size >= self.max_bytes {
            // Reset: truncate and restart
            drop(file);
            std::fs::write(&self.path, b"")?;
            let new_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            *self.file.lock().unwrap() = new_file;
            *size = buf.len() as u64;
            file = self.file.lock().unwrap();
        }

        file.write_all(buf)
    }
}

// tracing-subscriber needs `io::Write + Send + Sync + 'static`
impl io::Write for &CappedFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_all_impl(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Ok(mut f) = self.file.lock() {
            f.flush()
        } else {
            Ok(())
        }
    }
}

/// A clonable, `Send + Sync` wrapper around `CappedFileWriter` suitable for
/// use as a tracing-subscriber writer.
pub struct SharedFileWriter(pub std::sync::Arc<CappedFileWriter>);

impl Clone for SharedFileWriter {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedFileWriter {
    type Writer = SharedFileWriterGuard;

    fn make_writer(&'a self) -> Self::Writer {
        SharedFileWriterGuard(self.0.clone())
    }
}

pub struct SharedFileWriterGuard(std::sync::Arc<CappedFileWriter>);

impl Write for SharedFileWriterGuard {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_all_impl(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Ok(mut f) = self.0.file.lock() {
            f.flush()
        } else {
            Ok(())
        }
    }
}
