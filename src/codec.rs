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
    encode_with_capacity(value, max_bytes, 0)
}

pub(crate) fn encode_with_capacity<T: Serialize>(
    value: &T,
    max_bytes: usize,
    capacity: usize,
) -> Result<Vec<u8>> {
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
        bytes: Vec::with_capacity(capacity),
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

/// Private preflight for stable borrowed SDK records; public encode stays single-pass.
pub(crate) fn measure<T: Serialize>(value: &T, max_bytes: usize) -> Result<usize> {
    struct Counter {
        size: usize,
        limit: usize,
        failure: Option<Error>,
        depth: json::DepthScanner,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.failure.is_none() {
                self.failure = if bytes.len() > self.limit.saturating_sub(self.size) {
                    Some(Error::Capacity("record bytes"))
                } else {
                    self.depth.scan(bytes).err()
                };
            }
            if self.failure.is_some() {
                return Err(std::io::Error::other("JSON output limit exceeded"));
            }
            self.size += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter {
        size: 0,
        limit: max_bytes,
        failure: None,
        depth: json::DepthScanner::default(),
    };
    let mut serializer = serde_json::Serializer::new(&mut counter);
    let result = value.serialize(serde_stacker::Serializer::new(&mut serializer));
    if let Some(error) = counter.failure {
        return Err(error);
    }
    result.map_err(|_| Error::Protocol("cannot encode JSON".into()))?;
    Ok(counter.size)
}

pub struct JsonlReader<R> {
    reader: R,
    buffer: Vec<u8>,
    start: usize,
    scanned: usize,
    #[cfg(test)]
    scanned_bytes: usize,
    max_bytes: usize,
}
impl<R: AsyncRead + Unpin> JsonlReader<R> {
    pub fn new(reader: R, max_bytes: usize) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
            start: 0,
            scanned: 0,
            #[cfg(test)]
            scanned_bytes: 0,
            max_bytes,
        }
    }
    /// Returns parsed JSON and its UTF-8 wire size, excluding LF and optional CR.
    /// A nonempty unterminated final record is a protocol error, never accepted.
    pub async fn next(&mut self) -> Result<Option<(Value, usize)>> {
        loop {
            let pos = self.buffer[self.scanned..].iter().position(|b| *b == b'\n');
            #[cfg(test)]
            {
                self.scanned_bytes += pos.map_or(self.buffer.len() - self.scanned, |p| p + 1);
            }
            if let Some(pos) = pos {
                let end = self.scanned + pos;
                let record = &self.buffer[self.start..end];
                let record = record.strip_suffix(b"\r").unwrap_or(record);
                if record.len() > self.max_bytes {
                    return Err(Error::Protocol("record byte limit exceeded".into()));
                }
                let value = json::parse(record)?;
                let size = record.len();
                self.start = end + 1;
                self.scanned = self.start;
                return Ok(Some((value, size)));
            }
            self.scanned = self.buffer.len();
            if self.buffer.len() - self.start > self.max_bytes.saturating_add(1) {
                return Err(Error::Protocol("record byte limit exceeded".into()));
            }
            if self.start > 0 {
                self.buffer.drain(..self.start);
                self.scanned -= self.start;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_generic_encoding_invokes_serializer_once() {
        struct Once(std::cell::Cell<bool>);
        impl serde::Serialize for Once {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                assert!(!self.0.replace(true), "public serializer invoked twice");
                serializer.serialize_bool(true)
            }
        }
        assert_eq!(
            encode(&Once(std::cell::Cell::new(false)), 4).unwrap(),
            b"true\n"
        );
    }
    #[tokio::test]
    async fn large_fragmented_and_coalesced_records_scan_each_byte_once() {
        let first = format!("\"{}\"\r\n", "A".repeat(1024 * 1024));
        let wire = format!("{first}true\n");
        let mut reader = JsonlReader::new(wire.as_bytes(), first.len());
        assert!(reader.next().await.unwrap().is_some());
        assert_eq!(reader.next().await.unwrap().unwrap().0, Value::Bool(true));
        assert!(reader.next().await.unwrap().is_none());
        assert_eq!(reader.scanned_bytes, wire.len());
    }
    #[tokio::test]
    async fn cancelled_read_keeps_scan_position() {
        use tokio::io::AsyncWriteExt;
        let (mut writer, input) = tokio::io::duplex(64);
        writer.write_all(b"\"AAAA").await.unwrap();
        let mut reader = JsonlReader::new(input, 64);
        let mut next = Box::pin(reader.next());
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(next.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        drop(next);
        assert_eq!(reader.scanned_bytes, 5);
        writer.write_all(b"BBBB\"\n").await.unwrap();
        assert_eq!(
            reader.next().await.unwrap().unwrap().0,
            Value::String("AAAABBBB".into())
        );
        assert_eq!(reader.scanned_bytes, 11);
    }
}
