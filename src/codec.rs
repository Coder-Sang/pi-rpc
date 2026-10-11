//! Strict LF JSONL. CR is stripped only immediately before LF.
use crate::{Error, Result, json};
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Maximum JSON object/array nesting. The root container counts as one.
pub const MAX_JSON_DEPTH: usize = json::MAX_DEPTH;

/// Serializes at most max_bytes of JSON (the LF terminator is excluded).
/// Stops the serializer on its first write exceeding the budget.
pub fn encode<T: Serialize>(value: &T, max_bytes: usize) -> Result<Vec<u8>> {
    struct BoundedWriter {
        bytes: Vec<u8>,
        limit: usize,
        failure: Option<Error>,
        depth: json::DepthScanner,
    }
    impl std::io::Write for BoundedWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.failure.is_none() {
                self.failure = if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                    Some(Error::Capacity("record bytes"))
                } else {
                    self.depth.scan(bytes).err()
                };
            }
            if self.failure.is_some() {
                return Err(std::io::Error::other("JSON output limit exceeded"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = BoundedWriter {
        bytes: Vec::new(),
        limit: max_bytes,
        failure: None,
        depth: json::DepthScanner::default(),
    };
    let mut serializer = serde_json::Serializer::new(&mut writer);
    let result = value.serialize(serde_stacker::Serializer::new(&mut serializer));
    if let Some(error) = writer.failure {
        return Err(error);
    }
    result.map_err(|_| Error::Protocol("cannot encode JSON".into()))?;
    writer.bytes.push(b'\n');
    Ok(writer.bytes)
}

pub struct JsonlReader<R> {
    reader: R,
    buffer: Vec<u8>,
    start: usize,
    max_bytes: usize,
}
impl<R: AsyncRead + Unpin> JsonlReader<R> {
    pub fn new(reader: R, max_bytes: usize) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
            start: 0,
            max_bytes,
        }
    }
    /// Returns parsed JSON and its UTF-8 wire size, excluding LF and optional CR.
    /// A nonempty unterminated final record is a protocol error, never accepted.
    pub async fn next(&mut self) -> Result<Option<(Value, usize)>> {
        loop {
            if let Some(pos) = self.buffer[self.start..].iter().position(|b| *b == b'\n') {
                let end = self.start + pos;
                let record = &self.buffer[self.start..end];
                let record = record.strip_suffix(b"\r").unwrap_or(record);
                if record.len() > self.max_bytes {
                    return Err(Error::Protocol("record byte limit exceeded".into()));
                }
                let value = json::parse(record)?;
                let size = record.len();
                self.start = end + 1;
                return Ok(Some((value, size)));
            }
            if self.buffer.len() - self.start > self.max_bytes.saturating_add(1) {
                return Err(Error::Protocol("record byte limit exceeded".into()));
            }
            if self.start > 0 {
                self.buffer.drain(..self.start);
                self.start = 0;
            }
            let mut chunk = [0u8; 4096];
            let n = self
                .reader
                .read(&mut chunk)
                .await
                .map_err(|e| Error::Transport(e.to_string()))?;
            if n == 0 {
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err(Error::Protocol("unterminated record at EOF".into()))
                };
            }
            self.buffer.extend_from_slice(&chunk[..n]);
        }
    }
}
