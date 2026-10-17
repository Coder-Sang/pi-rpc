//! Strict LF JSONL. CR is stripped only immediately before LF.
use crate::{Error, Result, json};
use serde::Serialize;
use serde_json::Value;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncReadExt};

const READ_CHUNK_BYTES: usize = 4096;

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
            if let Some(record) = self.buffered_record()? {
                return Ok(Some(record));
            }
            let mut chunk = [0u8; READ_CHUNK_BYTES];
            let n = self
                .reader
                .read(&mut chunk)
                .await
                .map_err(|e| Error::Transport(e.to_string()))?;
            if n == 0 {
                self.check_eof()?;
                return Ok(None);
            }
            self.buffer.extend_from_slice(&chunk[..n]);
        }
    }

    /// Terminal-only validation: buffered records plus at most one chunk of
    /// immediately readable input. An unfinished record permits one additional
    /// byte of EOF lookahead, without further validation. Never await or dispatch.
    pub(crate) fn check_ready_at_exit(
        &mut self,
        cx: &mut Context<'_>,
        mut validate: impl FnMut(Value) -> Result<()>,
    ) -> Result<()> {
        let mut allowance = READ_CHUNK_BYTES;
        loop {
            while let Some((value, _)) = self.buffered_record()? {
                validate(value)?;
            }
            let eof_probe = allowance == 0;
            if eof_probe && self.buffer.is_empty() {
                return Ok(());
            }
            let mut chunk = [0u8; READ_CHUNK_BYTES];
            let read_limit = if eof_probe { 1 } else { allowance };
            let mut buffer = tokio::io::ReadBuf::new(&mut chunk[..read_limit]);
            match Pin::new(&mut self.reader).poll_read(cx, &mut buffer) {
                Poll::Pending => return Ok(()),
                Poll::Ready(Err(error)) => return Err(Error::Transport(error.to_string())),
                Poll::Ready(Ok(())) => {
                    let bytes = buffer.filled();
                    if bytes.is_empty() {
                        return self.check_eof();
                    }
                    self.buffer.extend_from_slice(bytes);
                    // Only an empty read establishes EOF. Keep any lookahead
                    // byte, but never parse it or poll again past the allowance.
                    if eof_probe {
                        return Ok(());
                    }
                    allowance -= bytes.len();
                }
            }
        }
    }

    fn check_eof(&self) -> Result<()> {
        if self.buffer.len() != self.start {
            Err(Error::Protocol("unterminated record at EOF".into()))
        } else {
            Ok(())
        }
    }

    fn buffered_record(&mut self) -> Result<Option<(Value, usize)>> {
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
        Ok(None)
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

#[cfg(test)]
mod exit_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct AlwaysReady(Arc<AtomicUsize>);
    impl AsyncRead for AlwaysReady {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buffer: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            const RECORD: &[u8] = b"{\"type\":\"agent_start\"}\n";
            let offset = self.0.fetch_add(1, Ordering::SeqCst);
            buffer.put_slice(&[RECORD[offset % RECORD.len()]]);
            Poll::Ready(Ok(()))
        }
    }
    #[test]
    fn terminal_ready_validation_has_a_byte_bound_even_for_tiny_continuous_reads() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut reader = JsonlReader::new(AlwaysReady(count.clone()), 65536);
        let mut records = 0;
        reader
            .check_ready_at_exit(
                &mut Context::from_waker(std::task::Waker::noop()),
                |value| {
                    records += 1;
                    crate::protocol::decode_record(value).map(drop)
                },
            )
            .unwrap();
        // The unfinished record permits exactly one extra byte of EOF
        // lookahead, never another round of continuous validation.
        assert_eq!(count.load(Ordering::SeqCst), READ_CHUNK_BYTES + 1);
        assert_eq!(
            records,
            READ_CHUNK_BYTES / b"{\"type\":\"agent_start\"}\n".len()
        );
    }
    #[test]
    fn terminal_eof_probe_does_not_validate_or_discard_lookahead() {
        for suffix in [&b""[..], &b"\n"[..], &b"x\n"[..]] {
            let mut wire = b"{\"type\":\"agent_start\"}".to_vec();
            wire.resize(READ_CHUNK_BYTES, b' ');
            wire.extend_from_slice(suffix);
            let mut reader = JsonlReader::new(wire.as_slice(), 8192);
            let mut records = 0;
            let result = reader.check_ready_at_exit(
                &mut Context::from_waker(std::task::Waker::noop()),
                |_| {
                    records += 1;
                    Ok(())
                },
            );
            if suffix.is_empty() {
                assert_eq!(
                    result,
                    Err(Error::Protocol("unterminated record at EOF".into()))
                );
            } else {
                assert_eq!(result, Ok(()));
            }
            assert_eq!(records, 0, "lookahead must not extend validation");
            let consumed = READ_CHUNK_BYTES + usize::from(!suffix.is_empty());
            assert_eq!(reader.buffer, wire[..consumed]);
            assert_eq!(reader.reader, &wire[consumed..]);
        }
    }
    #[test]
    fn terminal_ready_validation_checks_depth_boundaries_on_two_mib_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (arrays, valid) in [(511, true), (512, false)] {
                    let bytes = format!(
                        "{{\"type\":\"future\",\"data\":{}0{}}}\n",
                        "[".repeat(arrays),
                        "]".repeat(arrays)
                    );
                    let mut reader = JsonlReader::new(bytes.as_bytes(), 4096);
                    let checked = reader.check_ready_at_exit(
                        &mut Context::from_waker(std::task::Waker::noop()),
                        |value| crate::protocol::decode_record(value).map(drop),
                    );
                    assert_eq!(checked.is_ok(), valid);
                    if !valid {
                        assert!(matches!(checked, Err(Error::Protocol(_))));
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
