//! One external command, rendered for logs and job summaries.
//!
//! Execution itself belongs to the host, but how a command is described must not differ
//! between hosts: the same plan has to produce the same readable line in a log, a bug
//! report and a support screenshot.

/// Program and arguments for one invocation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Command {
    pub program: String,
    pub arguments: Vec<String>,
}

impl Command {
    /// Starts a command for `program`.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            arguments: Vec::new(),
        }
    }

    /// Appends one argument.
    pub fn arg(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());
        self
    }

    /// Appends several arguments in order.
    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.arguments.extend(arguments.into_iter().map(Into::into));
        self
    }

    /// One-line form with unsafe arguments quoted.
    pub fn render(&self) -> String {
        let mut parts = Vec::with_capacity(self.arguments.len() + 1);
        parts.push(quote(&self.program));
        parts.extend(self.arguments.iter().map(|argument| quote(argument)));
        parts.join(" ")
    }
}

/// Quotes an argument only when it contains characters a shell would split on.
pub fn quote(argument: &str) -> String {
    let safe = !argument.is_empty()
        && argument
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_./\\:=,".contains(character));
    if safe {
        return argument.to_owned();
    }

    format!("\"{}\"", argument.replace('"', "\\\""))
}
