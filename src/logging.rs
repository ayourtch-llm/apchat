use std::fs::File;
use std::io::{Write, BufWriter};
use std::path::PathBuf;

pub struct ConversationLogger {
    // Buffered writer for efficiency
    writer: Option<BufWriter<File>>,
}

impl ConversationLogger {
    /// Create a new logger that writes to a file named `conversation.log`
    /// inside the provided working directory.
    ///
    /// This function is async to match the usage in `main.rs`, but the
    /// underlying file operations are synchronous because they are fast and
    /// performed only once during initialization.
    pub async fn new(work_dir: &PathBuf) -> Result<Self, std::io::Error> {
        let log_path = work_dir.join("conversation.log");
        // Ensure the parent directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(&log_path)?;
        let writer = BufWriter::new(file);
        Ok(Self { writer: Some(writer) })
    }

    /// Log a message.
    ///
    /// The original code expected a simple `log(message)` method, but the
    /// caller now passes a role and a few extra arguments. To stay compatible
    /// we accept those extra parameters and ignore them – they are only used
    /// for future extensions.
    pub async fn log(
        &mut self,
        _role: &str,
        message: &str,
        _extra: Option<String>,
        _flag: bool,
    ) -> Result<(), std::io::Error> {
        if let Some(writer) = &mut self.writer {
            writeln!(writer, "{}", message)?;
            writer.flush()?;
        }
        Ok(())
    }

    /// Gracefully shutdown the logger, flushing any buffered data.
    pub async fn shutdown(&mut self) -> Result<(), std::io::Error> {
        if let Some(writer) = self.writer.take() {
            // Dropping the writer flushes the buffer; we explicitly flush
            // to surface any I/O errors before the object is dropped.
            let mut w = writer;
            w.flush()?;
        }
        Ok(())
    }
}