use openqg_domain::{agent_error, Result};
use std::path::Path;

const OPEN_PREFIX: &str = "<<<ZYAL v1:daemon id=";
const OPEN_SUFFIX: &str = ">>>";
const CLOSE_PREFIX: &str = "<<<END_ZYAL id=";
const CLOSE_SUFFIX: &str = ">>>";
const ARM_PREFIX: &str = "ZYAL_ARM RUN_FOREVER id=";

#[derive(Debug)]
pub(crate) struct ParsedEnvelope {
    pub(crate) id: String,
    pub(crate) armed: bool,
    pub(crate) body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EnvelopeParseState {
    Start,
    Open {
        id: String,
        open_index: usize,
    },
    Closed {
        id: String,
        open_index: usize,
        close_index: usize,
    },
    Armed {
        id: String,
        open_index: usize,
        close_index: usize,
        arm_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EnvelopeParseError {
    MissingOpening,
    InvalidOpening,
    EmptyId,
    MissingClosing { id: String },
    MissingArm { id: String },
    ArmMismatch { id: String },
    TrailingContent { id: String },
}

pub(super) fn parse_zyal_envelope(text: &str, path: &Path) -> Result<ParsedEnvelope> {
    let normalized = text.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.lines().collect();

    EnvelopeParseState::Start
        .advance_open(&lines, path)?
        .advance_close(&lines, path)?
        .advance_arm(&lines, path)?
        .finish(&lines, path)
}

impl EnvelopeParseState {
    fn advance_open(self, lines: &[&str], path: &Path) -> Result<Self> {
        let Some(open_index) = first_content_line(lines) else {
            return Err(EnvelopeParseError::MissingOpening.into_error(path));
        };

        let open_line = lines[open_index].trim();
        if !open_line.starts_with(OPEN_PREFIX) || !open_line.ends_with(OPEN_SUFFIX) {
            return Err(EnvelopeParseError::InvalidOpening.into_error(path));
        }

        let id = open_line[OPEN_PREFIX.len()..open_line.len() - OPEN_SUFFIX.len()]
            .trim()
            .to_string();
        if id.is_empty() {
            return Err(EnvelopeParseError::EmptyId.into_error(path));
        }

        Ok(Self::Open { id, open_index })
    }

    fn advance_close(self, lines: &[&str], path: &Path) -> Result<Self> {
        let Self::Open { id, open_index } = self else {
            unreachable!("invalid envelope parse state");
        };

        let close_line = format!("{CLOSE_PREFIX}{id}{CLOSE_SUFFIX}");
        let Some(close_index) = find_exact_line(lines, open_index + 1, &close_line) else {
            return Err(EnvelopeParseError::MissingClosing { id: id.clone() }.into_error(path));
        };

        Ok(Self::Closed {
            id,
            open_index,
            close_index,
        })
    }

    fn advance_arm(self, lines: &[&str], path: &Path) -> Result<Self> {
        let Self::Closed {
            id,
            open_index,
            close_index,
        } = self
        else {
            unreachable!("invalid envelope parse state");
        };

        let arm_line = format!("{ARM_PREFIX}{id}");
        let Some(arm_index) = find_first_non_empty_line(lines, close_index + 1) else {
            return Err(EnvelopeParseError::MissingArm { id: id.clone() }.into_error(path));
        };
        if lines[arm_index].trim() != arm_line {
            return Err(EnvelopeParseError::ArmMismatch { id }.into_error(path));
        }

        Ok(Self::Armed {
            id,
            open_index,
            close_index,
            arm_index,
        })
    }

    fn finish(self, lines: &[&str], path: &Path) -> Result<ParsedEnvelope> {
        let Self::Armed {
            id,
            open_index,
            close_index,
            arm_index,
        } = self
        else {
            unreachable!("invalid envelope parse state");
        };

        for line in lines.iter().skip(arm_index + 1) {
            if !line.trim().is_empty() && !line.trim_start().starts_with('#') {
                return Err(EnvelopeParseError::TrailingContent { id }.into_error(path));
            }
        }

        Ok(ParsedEnvelope {
            id,
            armed: true,
            body: lines[open_index + 1..close_index].join("\n"),
        })
    }
}

impl EnvelopeParseError {
    fn into_error(self, path: &Path) -> openqg_domain::DomainError {
        match self {
            Self::MissingOpening => agent_error(
                "parse zyal document",
                format!("missing opening sentinel in {}", path.display()),
                vec![
                    "add the opening ZYAL sentinel".into(),
                    "compare against an existing runbook".into(),
                ],
                "docs/testing.md",
                "restore the opening ZYAL envelope and rerun just zyal-validate",
            ),
            Self::InvalidOpening => agent_error(
                "parse zyal document",
                format!("invalid opening sentinel in {}", path.display()),
                vec![
                    "use <<<ZYAL v1:daemon id=...>>>".into(),
                    "remove any vv1 or alternate sentinel".into(),
                ],
                "docs/testing.md",
                "rewrite the opening sentinel to the canonical v1 form",
            ),
            Self::EmptyId => agent_error(
                "parse zyal document",
                format!("opening sentinel id is empty in {}", path.display()),
                vec!["set a non-empty daemon id".into()],
                "docs/testing.md",
                "give the runbook a stable sentinel id",
            ),
            Self::MissingClosing { id } => agent_error(
                "parse zyal document",
                format!("missing closing sentinel for {id} in {}", path.display()),
                vec![
                    "add the matching closing sentinel".into(),
                    "compare against an existing runbook".into(),
                ],
                "docs/testing.md",
                "restore the closing ZYAL sentinel and rerun just zyal-validate",
            ),
            Self::MissingArm { id } => agent_error(
                "parse zyal document",
                format!(
                    "missing trailing arm sentinel for {id} in {}",
                    path.display()
                ),
                vec!["append the ZYAL_ARM RUN_FOREVER line".into()],
                "docs/testing.md",
                "append the trailing arm sentinel and rerun just zyal-validate",
            ),
            Self::ArmMismatch { id } => agent_error(
                "parse zyal document",
                format!(
                    "trailing arm sentinel does not match id {id} in {}",
                    path.display()
                ),
                vec!["make the arm id match the opening sentinel".into()],
                "docs/testing.md",
                "fix the trailing ZYAL_ARM sentinel id",
            ),
            Self::TrailingContent { id } => agent_error(
                "parse zyal document",
                format!(
                    "unexpected trailing content after arm sentinel for {id} in {}",
                    path.display()
                ),
                vec!["remove trailing content after the arm sentinel".into()],
                "docs/testing.md",
                "keep the ZYAL arm line as the final non-comment line",
            ),
        }
    }
}

fn first_content_line(lines: &[&str]) -> Option<usize> {
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        return Some(index);
    }
    None
}

fn find_exact_line(lines: &[&str], start: usize, needle: &str) -> Option<usize> {
    for (index, line) in lines.iter().enumerate().skip(start) {
        if line.trim() == needle {
            return Some(index);
        }
    }
    None
}

fn find_first_non_empty_line(lines: &[&str], start: usize) -> Option<usize> {
    for (index, line) in lines.iter().enumerate().skip(start) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        return Some(index);
    }
    None
}
