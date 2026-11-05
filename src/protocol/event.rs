//! Payloads whose meaning is shared by all audited protocols.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompactionReason {
    Manual,
    Threshold,
    Overflow,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "source")]
pub enum SummarySource {
    #[serde(rename = "branchSummary")]
    BranchSummary,
    #[serde(rename = "compaction")]
    Compaction { reason: CompactionReason },
}
