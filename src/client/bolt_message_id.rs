//! Identified submissions reuse the ordinary command admission and writer.
use super::PiClient;
use crate::{
    BoltMessageIdV1, Error, Result,
    protocol::{
        Command, ImageContent, PromptDisposition, QueuedInputDisposition, StreamingBehavior,
        bolt_message_id_v1::{InputAcceptance, MessageId},
    },
};
use serde::de::DeserializeOwned;

impl PiClient<BoltMessageIdV1> {
    /// Submit an input with a caller-assigned ID; subscribe before awaiting this.
    /// Acceptance is distinct from consumption and assistant completion.
    pub fn prompt_with_message_id(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
        streaming_behavior: Option<StreamingBehavior>,
        message_id: MessageId,
    ) -> impl std::future::Future<Output = Result<InputAcceptance<PromptDisposition>>> + Send {
        let images = crate::owned::Owned::new(images);
        self.identified(
            Command::Prompt {
                message: message.into(),
                images: images.take(),
                streaming_behavior,
            },
            message_id,
        )
    }
    pub fn steer_with_message_id(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
        message_id: MessageId,
    ) -> impl std::future::Future<Output = Result<InputAcceptance<QueuedInputDisposition>>> + Send
    {
        let images = crate::owned::Owned::new(images);
        self.identified(
            Command::Steer {
                message: message.into(),
                images: images.take(),
            },
            message_id,
        )
    }
    pub fn follow_up_with_message_id(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
        message_id: MessageId,
    ) -> impl std::future::Future<Output = Result<InputAcceptance<QueuedInputDisposition>>> + Send
    {
        let images = crate::owned::Owned::new(images);
        self.identified(
            Command::FollowUp {
                message: message.into(),
                images: images.take(),
            },
            message_id,
        )
    }
    fn identified<T: DeserializeOwned>(
        &self,
        command: Command,
        expected: MessageId,
    ) -> impl std::future::Future<Output = Result<InputAcceptance<T>>> + Send {
        let response = self.command_record(
            command,
            Some(expected.shared()),
            self.shared.config.command_timeout,
        );
        async move {
            let response = response.await?;
            // Compare the borrowed echo before decoding; do not clone a large ID
            // for validation or keep the expected allocation through conversion.
            if response
                .data
                .as_ref()
                .and_then(|data| data.get("message_id"))
                .and_then(|id| id.as_str())
                != Some(expected.as_str())
            {
                return Err(Error::Protocol(
                    "message_id response missing or mismatched".into(),
                ));
            }
            drop(expected);
            response.decode()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, PiConnection, Transport};
    use std::{future::Future, task::Poll};

    fn fixture(
        config: Config,
    ) -> (
        PiClient<BoltMessageIdV1>,
        crate::PiEvents<BoltMessageIdV1>,
        PiConnection<BoltMessageIdV1>,
        tokio::io::DuplexStream,
    ) {
        let (local, peer) = tokio::io::duplex(1);
        let (r, w) = tokio::io::split(local);
        let (client, events, owner) =
            PiConnection::connect(Transport::new(r, w, "1.5.0").unwrap(), config).unwrap();
        (client, events, owner, peer)
    }
    fn refunds(client: &PiClient<BoltMessageIdV1>) {
        assert_eq!(client.shared.pending_count(), 0);
        assert_eq!(
            client.admission.available_permits(),
            client.shared.config.max_pending_commands
        );
        assert_eq!(
            client.budget.available_permits(),
            client.shared.config.max_writer_bytes
        );
        assert_eq!(
            client.writer.capacity(),
            client.shared.config.max_writer_records
        );
    }

    #[tokio::test]
    async fn rejected_identified_inputs_refund_all_capacity_and_large_id_ownership() {
        for state in 0..4 {
            let (client, _events, owner, _peer) = fixture(Config {
                max_record_bytes: 256,
                max_pending_commands: 1,
                ..Config::default()
            });
            let message_id = MessageId::new("x".repeat(1024 * 1024)).unwrap();
            let observed = std::sync::Arc::downgrade(&message_id.shared());
            let error = match state {
                0 => None,
                1 => {
                    client.shared.begin_close();
                    Some(Error::Closed)
                }
                2 => {
                    let e = Error::Protocol("terminal".into());
                    client.shared.finish(e.clone());
                    Some(e)
                }
                _ => None,
            };
            let held = if state == 3 {
                Some(client.admission.clone().acquire_owned().await.unwrap())
            } else {
                None
            };
            let result = client
                .prompt_with_message_id("input", None, None, message_id)
                .await;
            if let Some(error) = error {
                assert_eq!(result.unwrap_err(), error);
            } else {
                assert!(matches!(result, Err(Error::Capacity(_))));
            }
            assert!(
                observed.upgrade().is_none(),
                "rejected input retained its ID"
            );
            drop(held);
            refunds(&client);
            drop(owner);
        }
    }

    #[tokio::test]
    async fn identified_wire_budget_counts_escaped_id_and_lf_exactly() {
        let input = "\\\"\n🦀";
        let bytes=crate::codec::encode(&serde_json::json!({"id":"pi-rpc-0","type":"steer","message":"input","message_id":input}),4096).unwrap().len();
        for (record, writer, accepted) in [
            (bytes - 1, bytes, true),
            (bytes - 2, bytes, false),
            (bytes - 1, bytes - 1, false),
        ] {
            let (client, _events, owner, _peer) = fixture(Config {
                max_record_bytes: record,
                max_writer_bytes: writer,
                ..Config::default()
            });
            let mut command = Box::pin(client.steer_with_message_id(
                "input",
                None,
                MessageId::new(input).unwrap(),
            ));
            let result = std::future::poll_fn(|cx| Poll::Ready(command.as_mut().poll(cx))).await;
            if accepted {
                assert!(result.is_pending());
                assert_eq!(client.shared.pending_count(), 1);
                assert_eq!(client.budget.available_permits(), 0);
            } else {
                assert!(matches!(result, Poll::Ready(Err(Error::Capacity(_)))));
                refunds(&client);
            }
            drop(command);
            drop(owner);
            // Background writer tasks release their reservation when cancelled.
        }
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_and_cancelled_identified_waiters_release_pending_without_retry() {
        use tokio::io::AsyncWriteExt;
        for cancel in [false, true] {
            let (client, mut events, mut owner, mut peer) = fixture(Config {
                command_timeout: std::time::Duration::from_secs(1),
                ..Config::default()
            });
            let message_id = MessageId::new("A").unwrap();
            let weak = std::sync::Arc::downgrade(&message_id.shared());
            let mut command = Box::pin(client.follow_up_with_message_id("input", None, message_id));
            assert!(
                std::future::poll_fn(|cx| Poll::Ready(command.as_mut().poll(cx)))
                    .await
                    .is_pending()
            );
            assert_eq!(client.shared.pending_count(), 1);
            if !cancel {
                assert_eq!(command.as_mut().await, Err(Error::Timeout));
            }
            drop(command);
            assert!(weak.upgrade().is_none());
            assert_eq!(client.shared.pending_count(), 0);
            assert_eq!(client.admission.available_permits(), 128);
            // Terminating malformed input must refund a partially blocked write.
            peer.write_all(b"!\n").await.unwrap();
            let error = owner.wait_terminal().await;
            assert!(matches!(error, Error::Protocol(_)));
            refunds(&client);
            assert_eq!(events.recv().await, Some(Err(error)));
            assert_eq!(events.recv().await, None);
        }
    }
}
