use crate::config::Config;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A persistent session record stored on disk.
///
/// The `config` field is a frozen snapshot of the configuration at the time
/// the session was started (after capability phases have been merged in).
/// Commands that reuse this session can skip the fresh config load and
/// `merge_capability_phases` step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Short random hex ID, e.g. `"a3f7c2"`
    pub id: String,
    /// Lima VM name
    pub vm_name: String,
    /// Template name used to create this session's VM
    pub template_name: String,
    /// Project root at session-start time
    pub project_root: PathBuf,
    /// Creation timestamp (UTC)
    pub created_at: DateTime<Utc>,
    /// Frozen config snapshot (capability phases already merged)
    pub config: Config,
}
