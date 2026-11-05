//! Synthetic Release measurements; no timing assertions, Pi or model calls.
use pi_rpc::{
    Pi14,
    codec::{self, JsonlReader},
    protocol::{Command, MessageContent, decode_record},
    session_file::{SessionFileBuilder, user_message},
};
use std::{hint::black_box, time::Instant};
const ISO: &str = "2026-11-04T00:00:00Z";
const SAMPLES: usize = 7;
fn report(name: &str, records: usize, mut samples: Vec<u128>) {
    samples.sort();
    println!(
        "{{\"case\":\"{name}\",\"records\":{records},\"samples\":{SAMPLES},\"median_ns\":{}}}",
        samples[SAMPLES / 2]
    );
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let value = serde_json::json!({"type":"tool_execution_start","toolCallId":"c","toolName":"synthetic","args":{"path":"/project/synthetic","text":"x".repeat(2048)}});
    let wire = codec::encode(&value, 8192)?;
    let mut samples = Vec::new();
    for _ in 0..SAMPLES {
        let started = Instant::now();
        for _ in 0..10_000 {
            let (value, _) = JsonlReader::new(wire.as_slice(), 8192)
                .next()
                .await?
                .unwrap();
            drop(black_box(decode_record::<Pi14>(value)?));
        }
        samples.push(started.elapsed().as_nanos());
    }
    report("event_parse_decode_drop", 10_000, samples);
    let command = Command::Prompt {
        message: "synthetic text".repeat(256),
        images: None,
        streaming_behavior: None,
    };
    let mut samples = Vec::new();
    for _ in 0..SAMPLES {
        let started = Instant::now();
        for _ in 0..10_000 {
            drop(black_box(codec::encode(&command, 8192)?));
        }
        samples.push(started.elapsed().as_nanos());
    }
    report("command_encode", 10_000, samples);
    for count in [1_000, 2_000, 4_000] {
        let mut samples = Vec::new();
        for _ in 0..SAMPLES {
            let started = Instant::now();
            let mut builder = SessionFileBuilder::new("synthetic", ISO, "/project")?;
            for index in 0..count {
                builder.append_message(
                    index.to_string(),
                    (index > 0).then(|| (index - 1).to_string()),
                    ISO,
                    user_message(MessageContent::Text("synthetic text".into()), 1)?,
                )?;
            }
            drop(black_box(builder.finish()?));
            samples.push(started.elapsed().as_nanos());
        }
        report("session_builder", count, samples);
    }
    Ok(())
}
