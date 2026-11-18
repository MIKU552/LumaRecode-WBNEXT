use luma_core::execution::{Outcome, ProcessRunner, ProgressEvent, Runner, run, run_with_progress};
use luma_core::process::Command;

struct Fake {
    script: Vec<Outcome>,
    calls: usize,
}

impl Runner for Fake {
    fn run(&mut self, _: &Command) -> Outcome {
        self.calls += 1;
        self.script.remove(0)
    }
}

fn ok() -> Outcome {
    Outcome::Succeeded {
        exit_code: 0,
        stdout: String::new(),
        stderr_tail: String::new(),
    }
}

fn failed(message: &str) -> Outcome {
    Outcome::Failed {
        exit_code: Some(2),
        message: message.to_owned(),
    }
}

#[test]
fn every_step_runs_when_all_succeed() {
    let mut fake = Fake {
        script: vec![ok(), ok()],
        calls: 0,
    };
    let steps = [
        Command::new("ffmpeg").arg("-version"),
        Command::new("ffprobe"),
    ];
    let report = run(&mut fake, &steps);

    assert!(report.succeeded());
    assert_eq!(report.outcomes.len(), 2);
    assert_eq!(fake.calls, 2);
    assert!(!report.stopped_early);
}

#[test]
fn the_first_failure_stops_the_sequence() {
    let mut fake = Fake {
        script: vec![ok(), failed("encoder missing"), ok()],
        calls: 0,
    };
    let steps = [Command::new("a"), Command::new("b"), Command::new("c")];
    let report = run(&mut fake, &steps);

    assert!(!report.succeeded());
    assert!(report.stopped_early);
    assert_eq!(fake.calls, 2);
    match report.failure() {
        Some(Outcome::Failed { message, .. }) => assert_eq!(message, "encoder missing"),
        other => panic!("unexpected failure: {other:?}"),
    }
}

#[test]
fn a_failing_last_step_is_not_reported_as_stopped_early() {
    let mut fake = Fake {
        script: vec![failed("last")],
        calls: 0,
    };
    let report = run(&mut fake, &[Command::new("only")]);

    assert!(!report.succeeded());
    assert!(!report.stopped_early);
}

#[test]
fn the_process_runner_reports_a_real_process() {
    let Ok(cargo) = std::env::var("CARGO") else {
        return;
    };
    let mut runner = ProcessRunner;
    let outcome = runner.run(&Command::new(cargo).arg("--version"));
    assert!(outcome.succeeded(), "{outcome:?}");

    let missing = runner.run(&Command::new("luma-recode-missing-binary"));
    assert!(!missing.succeeded());
}

#[test]
fn a_sequence_reports_one_event_per_step() {
    let mut fake = Fake {
        script: vec![ok(), ok(), ok()],
        calls: 0,
    };
    let steps = [Command::new("a"), Command::new("b"), Command::new("c")];
    let mut seen: Vec<ProgressEvent> = Vec::new();
    let report = run_with_progress(&mut fake, &steps, 7, &mut |event| seen.push(event));

    assert!(report.succeeded());
    assert_eq!(seen.len(), 3);
    assert_eq!(
        seen.iter().map(|event| event.sequence).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    // Every event names its task, so two sequences can share a sink.
    assert!(
        seen.iter()
            .all(|event| event.task_id == 7 && event.total == 3)
    );
    assert!((seen[0].fraction - 1.0 / 3.0).abs() < 1e-6);
    assert_eq!(seen.last().map(|event| event.fraction), Some(1.0));
}

#[test]
fn a_failing_step_is_named_in_its_own_event() {
    let mut fake = Fake {
        script: vec![ok(), failed("boom")],
        calls: 0,
    };
    let steps = [Command::new("a"), Command::new("b"), Command::new("c")];
    let mut seen: Vec<ProgressEvent> = Vec::new();
    let report = run_with_progress(&mut fake, &steps, 3, &mut |event| seen.push(event));

    assert!(!report.succeeded());
    assert!(report.stopped_early);
    // The failure reports itself; the steps after it never run and never report.
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[1].step, 1);
    assert!((seen[1].fraction - 2.0 / 3.0).abs() < 1e-6);
}
