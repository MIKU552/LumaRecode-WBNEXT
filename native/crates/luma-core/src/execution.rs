//! Sequential execution of planned commands.
//!
//! Planning is pure and lives elsewhere; this module is the small state machine that runs
//! the steps in order and reports what happened. The runner is a trait so tests can script
//! outcomes, and the host can decide how a process is actually started.

use std::process::{Command as ProcessCommand, Stdio};

use crate::process::Command;

/// Result of one step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The process exited with status zero.
    Succeeded {
        exit_code: i32,
        /// Last part of stderr, where FFmpeg writes progress and warnings.
        stderr_tail: String,
    },
    /// The process exited non-zero or could not be started.
    Failed {
        exit_code: Option<i32>,
        message: String,
    },
}

impl Outcome {
    /// True when the step succeeded.
    pub fn succeeded(&self) -> bool {
        matches!(self, Self::Succeeded { .. })
    }
}

/// Starts one command and waits for it.
pub trait Runner {
    /// Runs the command to completion.
    fn run(&mut self, command: &Command) -> Outcome;
}

/// What happened across a sequence of steps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
    /// A failure stopped the sequence before the remaining steps ran.
    pub stopped_early: bool,
}

impl Report {
    /// True when every step ran and succeeded.
    pub fn succeeded(&self) -> bool {
        !self.stopped_early && self.outcomes.iter().all(Outcome::succeeded)
    }

    /// First failing step, if any.
    pub fn failure(&self) -> Option<&Outcome> {
        self.outcomes.iter().find(|outcome| !outcome.succeeded())
    }
}

/// Runs the steps in order and stops at the first failure.
///
/// Stopping is deliberate: a later FFmpeg invocation usually depends on the previous one,
/// and continuing after a failure produces output that looks finished but is not.
pub fn run(runner: &mut dyn Runner, steps: &[Command]) -> Report {
    let mut report = Report::default();
    for step in steps {
        let outcome = runner.run(step);
        let failed = !outcome.succeeded();
        report.outcomes.push(outcome);
        if failed {
            report.stopped_early = report.outcomes.len() < steps.len();
            break;
        }
    }

    report
}

/// Spawns real processes with stdin closed and both streams captured.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessRunner;

impl Runner for ProcessRunner {
    fn run(&mut self, command: &Command) -> Outcome {
        let mut process = ProcessCommand::new(&command.program);
        process
            .args(&command.arguments)
            .stdin(Stdio::null());
        match process.output() {
            Ok(output) => {
                let code = output.status.code().unwrap_or(-1);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    Outcome::Succeeded {
                        exit_code: code,
                        stderr_tail: tail(&stderr),
                    }
                } else {
                    Outcome::Failed {
                        exit_code: Some(code),
                        message: tail(&stderr),
                    }
                }
            }
            Err(error) => Outcome::Failed {
                exit_code: None,
                message: error.to_string(),
            },
        }
    }
}

/// Keeps the last few hundred characters; a full FFmpeg log does not fit in a report.
fn tail(text: &str) -> String {
    const LIMIT: usize = 400;
    let length = text.len();
    if length <= LIMIT {
        return text.to_owned();
    }

    let mut start = length - LIMIT;
    while start < length && !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_owned()
}
