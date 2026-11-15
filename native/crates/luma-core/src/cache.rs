//! Extraction-cache identity and retention, per plan §10.4.
//!
//! The rules here are pure: callers supply ages, sizes and in-use flags that they read
//! from the filesystem, and receive a keep/remove decision per entry. Nothing in this
//! module touches the disk, so the policy can be tested without creating a cache and
//! without depending on the clock.

use crate::error::{codes, CoreError, CoreResult};

/// Identifies one published build.
///
/// The key deliberately excludes the display version and the executable name: renaming a
/// release or bumping its marketing version must reuse the same extraction instead of
/// orphaning it, while any change to the code, toolchain or payload must produce a new key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildIdentity {
    /// Source revision the binaries were built from.
    pub source_commit: String,
    /// Compiler and runtime versions, already reduced to a short token.
    pub toolchain: String,
    /// Digest of the extracted payload.
    pub payload_digest: String,
}

impl BuildIdentity {
    /// Stable directory name for this build.
    pub fn key(&self) -> String {
        let commit = &self.source_commit[..self.source_commit.len().min(12)];
        let digest = &self.payload_digest[..self.payload_digest.len().min(8)];
        format!("{}-{}-{}", sanitize(commit), sanitize(&self.toolchain), sanitize(digest))
    }

    /// Rejects identities that would collapse different builds onto one directory.
    pub fn validate(&self) -> CoreResult<()> {
        if self.source_commit.trim().is_empty() {
            return Err(CoreError::with_detail(codes::CACHE_IDENTITY, "commit"));
        }

        if self.toolchain.trim().is_empty() {
            return Err(CoreError::with_detail(codes::CACHE_IDENTITY, "toolchain"));
        }

        if self.payload_digest.trim().is_empty() {
            return Err(CoreError::with_detail(codes::CACHE_IDENTITY, "digest"));
        }

        Ok(())
    }
}

/// One extraction directory the application may reclaim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Key produced by [`BuildIdentity::key`] when the directory was created.
    pub key: String,
    /// Whole days since the directory was last used.
    pub age_days: u32,
    /// Size on disk in bytes.
    pub bytes: u64,
    /// True while a running instance still has the directory mapped.
    pub in_use: bool,
}

/// Retention limits. Zero disables the corresponding limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// How many reclaimable builds to keep beyond the current one.
    pub keep_recent: usize,
    /// Maximum age of a reclaimable build.
    pub max_age_days: u32,
    /// Maximum total size of everything that is kept.
    pub max_bytes: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            keep_recent: 2,
            max_age_days: 30,
            max_bytes: 2 * 1024 * 1024 * 1024,
        }
    }
}

/// Outcome for one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The build running right now.
    KeepCurrent,
    /// A directory another instance is using.
    KeepInUse,
    /// A recent build kept for rollback or fast restart.
    KeepRecent,
    /// Older than [`Policy::max_age_days`].
    RemoveTooOld,
    /// Beyond [`Policy::keep_recent`].
    RemoveOverCount,
    /// Needed to fit [`Policy::max_bytes`].
    RemoveOverBudget,
}

impl Decision {
    /// True when the caller may delete the directory.
    pub fn remove(self) -> bool {
        matches!(self, Self::RemoveTooOld | Self::RemoveOverCount | Self::RemoveOverBudget)
    }
}

/// Decides one action per entry, in input order.
///
/// The current build and directories in use are never removed. The newest
/// [`Policy::keep_recent`] entries survive the count pass, and the oldest remaining
/// entries are dropped first when the kept total exceeds the budget.
pub fn plan(entries: &[Entry], current: &str, policy: &Policy) -> Vec<(String, Decision)> {
    let mut decisions: Vec<Decision> = entries
        .iter()
        .map(|entry| {
            if entry.key == current {
                Decision::KeepCurrent
            } else if entry.in_use {
                Decision::KeepInUse
            } else if policy.max_age_days > 0 && entry.age_days > policy.max_age_days {
                Decision::RemoveTooOld
            } else {
                Decision::KeepRecent
            }
        })
        .collect();

    let mut recent: Vec<usize> = decisions
        .iter()
        .enumerate()
        .filter(|(_, decision)| **decision == Decision::KeepRecent)
        .map(|(index, _)| index)
        .collect();
    recent.sort_by_key(|index| entries[*index].age_days);
    for (rank, index) in recent.iter().enumerate() {
        if rank >= policy.keep_recent {
            decisions[*index] = Decision::RemoveOverCount;
        }
    }

    let mut kept: u64 = entries
        .iter()
        .zip(decisions.iter())
        .filter(|(_, decision)| !decision.remove())
        .map(|(entry, _)| entry.bytes)
        .sum();

    let mut reclaimable: Vec<usize> = decisions
        .iter()
        .enumerate()
        .filter(|(_, decision)| **decision == Decision::KeepRecent)
        .map(|(index, _)| index)
        .collect();
    reclaimable.sort_by_key(|index| std::cmp::Reverse(entries[*index].age_days));
    for index in reclaimable {
        if policy.max_bytes == 0 || kept <= policy.max_bytes {
            break;
        }

        decisions[index] = Decision::RemoveOverBudget;
        kept = kept.saturating_sub(entries[index].bytes);
    }

    entries
        .iter()
        .map(|entry| entry.key.clone())
        .zip(decisions)
        .collect()
}

/// Total size of the entries a plan would keep.
pub fn kept_bytes(entries: &[Entry], decisions: &[(String, Decision)]) -> u64 {
    entries
        .iter()
        .zip(decisions)
        .filter(|(_, pair)| !pair.1.remove())
        .map(|(entry, _)| entry.bytes)
        .sum()
}

/// Total size a plan would reclaim.
pub fn reclaimable_bytes(entries: &[Entry], decisions: &[(String, Decision)]) -> u64 {
    entries
        .iter()
        .zip(decisions)
        .filter(|(_, pair)| pair.1.remove())
        .map(|(entry, _)| entry.bytes)
        .sum()
}

/// Keeps a key to lowercase ASCII so it is a safe directory name on every host.
fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}
