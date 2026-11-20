use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub const MAX_LINES: usize = 10_000;
const MAX_READ: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub id: u64,
    pub received: OffsetDateTime,
    pub line: String,
    pub(crate) persisted: bool,
    pub(crate) paired: bool,
}

impl LogEntry {
    pub fn event(line: String, received: OffsetDateTime) -> Self {
        Self {
            id: 0,
            received,
            line,
            persisted: false,
            paired: false,
        }
    }

    fn disk(line: &str) -> Self {
        let (received, line) = line
            .split_once(' ')
            .and_then(|(stamp, message)| {
                OffsetDateTime::parse(stamp, &Rfc3339)
                    .ok()
                    .map(|at| (at, message))
            })
            .unwrap_or((OffsetDateTime::now_utc(), line));
        Self {
            id: 0,
            received,
            line: line.to_owned(),
            persisted: true,
            paired: false,
        }
    }

    pub fn matches(&self, tunnel: &str, query: &str) -> bool {
        (tunnel.is_empty()
            || self
                .line
                .split(|c: char| !c.is_alphanumeric() && !matches!(c, '-' | '_' | '.'))
                .any(|word| word == tunnel))
            && self.line.to_lowercase().contains(&query.to_lowercase())
    }

    pub fn display(&self) -> String {
        format!(
            "{} {:02}:{:02}:{:02} UTC {}",
            self.received.date(),
            self.received.hour(),
            self.received.minute(),
            self.received.second(),
            self.line
        )
    }
}

/// Incremental, bounded tail. Reopening each read handles rename rotation without holding old files.
pub struct LogTail {
    path: PathBuf,
    inode: Option<u64>,
    offset: u64,
    partial: Vec<u8>,
}

impl LogTail {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            inode: None,
            offset: 0,
            partial: Vec::new(),
        }
    }

    pub fn read(&mut self) -> std::io::Result<Vec<LogEntry>> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let meta = file.metadata()?;
        if self.inode != Some(meta.ino()) || meta.len() < self.offset {
            self.offset = 0;
            self.partial.clear();
            self.inode = Some(meta.ino());
        }
        let start = self.offset.max(meta.len().saturating_sub(MAX_READ));
        let skipped = start > self.offset;
        if skipped {
            self.partial.clear();
        }
        file.seek(SeekFrom::Start(start))?;
        let count = file.take(MAX_READ).read_to_end(&mut self.partial)?;
        self.offset = start.saturating_add(u64::try_from(count).unwrap_or(MAX_READ));
        let Some(end) = self.partial.iter().rposition(|b| *b == b'\n') else {
            if self.partial.len() > usize::try_from(MAX_READ).unwrap_or(2_097_152) {
                self.partial.clear();
            }
            return Ok(Vec::new());
        };
        let complete: Vec<_> = self.partial.drain(..=end).collect();
        let text = String::from_utf8_lossy(&complete);
        Ok(text
            .lines()
            .skip(usize::from(skipped))
            .filter(|line| !line.is_empty())
            .map(LogEntry::disk)
            .collect())
    }
}
