use super::*;
use crate::{
    PI_VERSION, Transport,
    client::{PiClient, PiConnection},
};

fn fixture(config: Config) -> (PiClient, PiConnection, tokio::io::DuplexStream) {
    let (local, peer) = tokio::io::duplex(1024);
    let (reader, writer) = tokio::io::split(local);
    let (client, _events, owner) =
        PiConnection::connect(Transport::new(reader, writer, PI_VERSION).unwrap(), config).unwrap();
    (client, owner, peer)
}

#[tokio::test]
async fn automatic_final_admission_prioritizes_terminal_then_close_and_refunds() {
    for max_writer_bytes in [1, 1024] {
        for terminal in [
            None,
            Some(Error::Transport("racing transport failure".into())),
        ] {
            let (client, owner, _peer) = fixture(Config {
                max_writer_bytes,
                ..Config::default()
            });
            assert_eq!(client.shared.should_cancel_ui(), Ok(true));
            let bytes = client.budget.available_permits();
            let slots = client.writer.capacity();
            let prepared = client.prepare_record(&true);
            assert_eq!(prepared.is_ok(), max_writer_bytes > 1);
            client.shared.begin_close();
            if let Some(error) = &terminal {
                client.shared.finish(error.clone());
            }
            let expected = terminal.map_or(Ok(()), Err);
            assert_eq!(client.shared.enqueue_auto_reply(prepared), expected);
            assert_eq!(client.budget.available_permits(), bytes);
            assert_eq!(client.writer.capacity(), slots);
            assert_eq!(client.shared.pending_count(), 0);
            drop(owner);
        }
    }
}

#[tokio::test]
async fn command_final_admission_after_close_or_terminal_refunds_every_reservation() {
    for terminal in [
        None,
        Some(Error::Transport("racing transport failure".into())),
    ] {
        let (client, owner, _peer) = fixture(Config {
            max_pending_commands: 1,
            ..Config::default()
        });
        client.shared.check_command_capacity().unwrap();
        let bytes = client.budget.available_permits();
        let slots = client.writer.capacity();
        let admission = client.admission.clone().try_acquire_owned().unwrap();
        let prepared = client.prepare_record(&true).unwrap();
        let (response, _rx) = oneshot::channel();
        client.shared.begin_close();
        if let Some(error) = &terminal {
            client.shared.finish(error.clone());
        }
        let expected = terminal.unwrap_or(Error::Closed);
        assert_eq!(
            client
                .shared
                .enqueue_command("test-id", "abort", response, admission, prepared)
                .unwrap_err(),
            expected
        );
        assert_eq!(client.budget.available_permits(), bytes);
        assert_eq!(client.writer.capacity(), slots);
        assert_eq!(client.admission.available_permits(), 1);
        assert_eq!(client.shared.pending_count(), 0);
        drop(owner);
    }
}
