//! Fake-only bulk construction timing. No filesystem or Pi/model access.
use pi_rpc::{
    protocol::MessageContent,
    session_file::{SessionFile, SessionFileBuilder, user_message},
};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};
const ISO: &str = "2026-10-11T00:00:00Z";
fn build(count: usize, incremental: bool) -> pi_rpc::Result<SessionFile> {
    let mut builder = SessionFileBuilder::new("bench", ISO, "/project")?;
    let mut file = SessionFile::new("bench", ISO, "/project");
    for i in 0..count {
        let parent = if i == 0 {
            None
        } else {
            Some((i - 1).to_string())
        };
        let message = user_message(MessageContent::Text("native message".into()), 1)?;
        if incremental {
            builder.append_message(i.to_string(), parent, ISO, message)?;
        } else {
            file.append_message(i.to_string(), parent, ISO, message)?;
        }
    }
    if incremental {
        builder.finish()
    } else {
        Ok(file)
    }
}
fn main() -> pi_rpc::Result<()> {
    for count in [1000, 2000, 4000] {
        let mut medians = Vec::new();
        for incremental in [false, true] {
            let mut samples: Vec<Duration> = Vec::new();
            for _ in 0..3 {
                let started = Instant::now();
                let file = black_box(build(count, incremental)?);
                samples.push(started.elapsed());
                assert_eq!(file.entries.len(), count);
            }
            samples.sort();
            medians.push(samples[1]);
        }
        println!(
            "messages={count} legacy_median={:?} builder_median={:?} samples=3",
            medians[0], medians[1]
        );
    }
    Ok(())
}
