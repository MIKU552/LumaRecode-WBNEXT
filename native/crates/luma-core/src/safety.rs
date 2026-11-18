//! Output-path safety rules, per plan §10.3.
//!
//! The caller supplies what it learned from the filesystem (existence, symlink status,
//! volume case behaviour); this module decides whether the write is allowed. Keeping the
//! rules pure means the same logic protects the CLI, the GUI and the tests, and that a
//! platform can report its own facts without the decision drifting between hosts.

use crate::error::{CoreError, CoreResult, codes};

/// What the host knows about one path before asking for a decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathFacts {
    /// Path as the user supplied it.
    pub path: String,
    /// True when something already exists at that path.
    pub exists: bool,
    /// True when the final component is a symbolic link or reparse point.
    pub symlink: bool,
    /// True when the volume compares names without case.
    pub case_insensitive: bool,
}

impl PathFacts {
    /// Convenience constructor for a path that does not exist yet.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            exists: false,
            symlink: false,
            case_insensitive: false,
        }
    }

    /// Comparable form: separators unified, redundant segments resolved, case folded
    /// when the volume ignores it.
    pub fn normalized(&self) -> String {
        let unified = self.path.replace('\\', "/");
        let mut segments: Vec<&str> = Vec::new();
        for segment in unified.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    segments.pop();
                }
                other => segments.push(other),
            }
        }

        let joined = segments.join("/");
        let prefix = if unified.starts_with('/') { "/" } else { "" };
        let normalized = format!("{prefix}{joined}");
        if self.case_insensitive {
            normalized.to_lowercase()
        } else {
            normalized
        }
    }
}

/// What the caller should do once the path is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing exists at the destination.
    Create,
    /// The user confirmed replacing the existing file.
    Replace,
}

impl Action {
    /// True when the caller must stage a temporary file and swap it in atomically.
    pub fn replace(self) -> bool {
        matches!(self, Self::Replace)
    }
}

/// Decides whether `output` may be written and how.
///
/// Rejections cover the three ways a write can destroy data: following a link the user did
/// not mean, overwriting one of the inputs, and replacing a file without confirmation.
pub fn check(output: &PathFacts, inputs: &[PathFacts], overwrite: bool) -> CoreResult<Action> {
    if output.symlink {
        return Err(CoreError::with_detail(
            codes::OUTPUT_SYMLINK,
            output.path.clone(),
        ));
    }

    let target = output.normalized();
    for input in inputs {
        if input.normalized() == target {
            return Err(CoreError::with_detail(
                codes::OUTPUT_SAME_AS_INPUT,
                input.path.clone(),
            ));
        }
    }

    if output.exists && !overwrite {
        return Err(CoreError::with_detail(
            codes::OUTPUT_EXISTS,
            output.path.clone(),
        ));
    }

    Ok(if output.exists {
        Action::Replace
    } else {
        Action::Create
    })
}
