//! Serialized writer, continuous reader and process-exit observation.
use super::{BufferedEvent, PiClient, UiPolicy, WriteMessage};
use crate::{
    Error, Result, codec,
    protocol::{Event, Record, decode_record},
};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    sync::{Semaphore, mpsc, watch},
};

pub(super) async fn write_loop(
    mut writer: crate::transport::Writer,
    mut rx: mpsc::Receiver<WriteMessage>,
) -> Result<()> {
    while let Some(msg) = rx.recv().await {
        match msg {
            WriteMessage::Record(out) => {
                let r = async {
                    writer.write_all(&out.bytes).await?;
                    writer.flush().await
                }
                .await
                .map_err(|e: std::io::Error| Error::Transport(e.to_string()));
                let _ = out.sent.send(r.clone());
                r?;
            }
            WriteMessage::Close => {
                writer
                    .shutdown()
                    .await
                    .map_err(|e| Error::Transport(e.to_string()))?;
                drop(writer);
                std::future::pending::<()>().await;
                return Ok(());
            }
        }
    }
    Ok(())
}
pub(super) async fn read_loop(
    reader: crate::transport::Reader,
    mut exit: Option<watch::Receiver<Option<Error>>>,
    client: PiClient,
    events: mpsc::Sender<BufferedEvent>,
    budget: Arc<Semaphore>,
) -> Result<()> {
    let mut reader = codec::JsonlReader::new(reader, client.shared.config.max_record_bytes);
    // Child exit and pipe EOF are independent: descendants can retain stdout.
    // After exit, drain ready records for at most 50ms, then fail even if a
    // descendant keeps producing output. An EOF gets 100ms exit-race grace.
    let mut exited: Option<(Error, tokio::time::Instant)> = None;
    loop {
        let record = if let Some((reason, deadline)) = &exited {
            tokio::select! { biased;
                _=tokio::time::sleep_until(*deadline)=> {
                    // Preserve ready protocol failures if scheduling crossed the
                    // deadline. This bounded check does not extend event delivery.
                    let checked = std::future::poll_fn(|cx| std::task::Poll::Ready(
                        reader.check_ready_at_exit(cx, |value| decode_record(value).map(drop))
                    )).await;
                    if let Err(error @ Error::Protocol(_)) = checked {
                        return Err(error);
                    }
                    return Err(reason.clone());
                },
                record=reader.next()=>record?,
            }
        } else {
            tokio::select! { biased;
                reason=wait_exit(&mut exit)=> {
                    exited=Some((reason,tokio::time::Instant::now()+Duration::from_millis(50)));
                    continue;
                }
                record=reader.next()=>record?,
            }
        };
        let Some((value, size)) = record else {
            if let Some((reason, _)) = exited {
                return Err(reason);
            }
            if let Some(rx) = exit.as_mut() {
                let reason = tokio::time::timeout(Duration::from_millis(100), async {
                    loop {
                        if let Some(e) = rx.borrow().clone() {
                            return Some(e);
                        }
                        if rx.changed().await.is_err() {
                            return None;
                        }
                    }
                })
                .await;
                if let Ok(Some(e)) = reason {
                    return Err(e);
                }
            }
            return Err(if client.shared.is_closing() {
                Error::Closed
            } else {
                Error::Eof
            });
        };
        let event = match decode_record(value)? {
            Record::Response(response) => match client.shared.dispatch_response(response)? {
                Some(response) => Event::UncorrelatedResponse(response),
                None => continue,
            },
            Record::Event(e) => e,
        };
        if let Event::Ui(u) = &event
            && u.method.is_dialog()
            && client.shared.config.ui_policy == UiPolicy::Cancel
        {
            // Enqueue without awaiting the writer, so the reader remains continuous.
            client.cancel_ui_if_open(u)?;
        }
        let permit = budget
            .clone()
            .try_acquire_many_owned(size as u32)
            .map_err(|_| Error::EventOverflow)?;
        events
            .try_send(BufferedEvent {
                event,
                _budget: permit,
            })
            .map_err(|_| Error::EventOverflow)?;
    }
}
async fn wait_exit(exit: &mut Option<watch::Receiver<Option<Error>>>) -> Error {
    if let Some(rx) = exit {
        loop {
            if let Some(e) = rx.borrow().clone() {
                return e;
            }
            if rx.changed().await.is_err() {
                break;
            }
        }
    }
    std::future::pending().await
}

#[cfg(test)]
mod tests;
