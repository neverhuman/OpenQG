use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairHint {
    pub purpose: String,
    pub reason: String,
    pub common_fixes: Vec<String>,
    pub docs_url: String,
    pub repair_hint: String,
}

impl RepairHint {
    pub fn new(
        purpose: impl Into<String>,
        reason: impl Into<String>,
        common_fixes: Vec<String>,
        docs_url: impl Into<String>,
        repair_hint: impl Into<String>,
    ) -> Self {
        Self {
            purpose: purpose.into(),
            reason: reason.into(),
            common_fixes,
            docs_url: docs_url.into(),
            repair_hint: repair_hint.into(),
        }
    }

    pub fn purpose(&self) -> &str {
        &self.purpose
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn common_fixes(&self) -> &[String] {
        &self.common_fixes
    }

    pub fn docs_url(&self) -> &str {
        &self.docs_url
    }

    pub fn repair_hint(&self) -> &str {
        &self.repair_hint
    }

    pub fn render(&self) -> String {
        let fixes = if self.common_fixes.is_empty() {
            String::from("none")
        } else {
            self.common_fixes.join("; ")
        };
        format!(
            "purpose={}; reason={}; common_fixes={}; docs_url={}; repair_hint={}",
            self.purpose, self.reason, fixes, self.docs_url, self.repair_hint
        )
    }
}

impl fmt::Display for RepairHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("{hint}")]
    Repair { hint: RepairHint },
}

impl DomainError {
    pub fn hint(&self) -> &RepairHint {
        match self {
            Self::Repair { hint } => hint,
        }
    }

    pub fn render(&self) -> String {
        self.hint().render()
    }
}

pub type Result<T> = std::result::Result<T, DomainError>;

pub fn agent_error(
    purpose: impl Into<String>,
    reason: impl Into<String>,
    common_fixes: Vec<String>,
    docs_url: impl Into<String>,
    repair_hint: impl Into<String>,
) -> DomainError {
    DomainError::Repair {
        hint: RepairHint::new(purpose, reason, common_fixes, docs_url, repair_hint),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_structured_error_fields() {
        let err = agent_error(
            "parse zyal",
            "missing end sentinel",
            vec!["add the terminator".into()],
            "docs/testing.md",
            "rerun just zyal-validate",
        );

        assert_eq!(err.hint().purpose(), "parse zyal");
        assert_eq!(err.hint().reason(), "missing end sentinel");
        assert_eq!(err.hint().common_fixes(), ["add the terminator"]);
        assert_eq!(err.hint().docs_url(), "docs/testing.md");
        assert_eq!(err.hint().repair_hint(), "rerun just zyal-validate");
        assert!(err.render().contains("purpose=parse zyal"));
    }
}
