//! Native mixed-history parse timings; no Pi or model access.
use pi_rpc::{
    protocol::{EntryData, MessageContent, SessionEntry},
    session_file::{SessionFile, SessionLimits, user_message},
};
use std::{hint::black_box, time::Instant};
fn main() -> pi_rpc::Result<()> {
    for count in [1000, 2000, 4000] {
        let mut file = SessionFile::new("bench", "t", "/p");
        file.append_message(
            "0",
            None,
            "t",
            user_message(MessageContent::Text("root".into()), 1)?,
        )?;
        for i in 1..count {
            file.entries.push(SessionEntry {
                id: i.to_string(),
                parent_id: Some((i - 1).to_string()),
                timestamp: "t".into(),
                data: EntryData::ContextEdit {
                    target_id: "0".into(),
                    replacement: None,
                },
            });
        }
        let bytes = file.encode(SessionLimits::default())?;
        let mut samples = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            let parsed = black_box(SessionFile::parse(
                black_box(&bytes),
                SessionLimits::default(),
            )?);
            samples.push(start.elapsed());
            assert_eq!(parsed.entries.len(), count);
        }
        samples.sort();
        println!("entries={count} parse_median={:?} samples=7", samples[3]);
    }
    Ok(())
}
