use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a stable JSON Path for a node or property in the document.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JsonPath(String);

impl JsonPath {
    pub fn root() -> Self {
        Self("$".to_string())
    }

    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn child(&self, key: &str) -> Self {
        if key.chars().all(|c| c.is_alphanumeric() || c == '_') && !key.is_empty() {
            Self(format!("{}.{}", self.0, key))
        } else {
            Self(format!("{}[\"{}\"]", self.0, key.replace('"', "\\\"")))
        }
    }

    pub fn index(&self, index: usize) -> Self {
        Self(format!("{}[{}]", self.0, index))
    }

    pub fn range(&self, start: usize, end: usize) -> Self {
        Self(format!("{}[{}..{}]", self.0, start, end))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parent(&self) -> Option<Self> {
        if self.0 == "$" || self.0.is_empty() {
            return None;
        }

        let dot_idx = self.0.rfind('.');
        let bracket_idx = self.0.rfind('[');

        let split_idx = match (dot_idx, bracket_idx) {
            (Some(d), Some(b)) => d.max(b),
            (Some(d), None) => d,
            (None, Some(b)) => b,
            (None, None) => return Some(Self::root()),
        };

        if split_idx == 0 || &self.0[..split_idx] == "$" {
            Some(Self::root())
        } else {
            Some(Self(self.0[..split_idx].to_string()))
        }
    }
}

impl Default for JsonPath {
    fn default() -> Self {
        Self::root()
    }
}

impl fmt::Display for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JsonPath({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_path_building() {
        let root = JsonPath::root();
        assert_eq!(root.as_str(), "$");

        let squad = root.child("squadName");
        assert_eq!(squad.as_str(), "$.squadName");

        let members = root.child("members");
        assert_eq!(members.as_str(), "$.members");

        let first_member = members.index(0);
        assert_eq!(first_member.as_str(), "$.members[0]");

        let powers = first_member.child("powers");
        assert_eq!(powers.as_str(), "$.members[0].powers");

        let special_key = root.child("special key with spaces");
        assert_eq!(special_key.as_str(), "$[\"special key with spaces\"]");
    }

    #[test]
    fn test_json_path_parent() {
        let path = JsonPath::root().child("members").index(0).child("name");
        assert_eq!(path.as_str(), "$.members[0].name");
        assert_eq!(path.parent().unwrap().as_str(), "$.members[0]");
        assert_eq!(
            path.parent().unwrap().parent().unwrap().as_str(),
            "$.members"
        );
        assert_eq!(
            path.parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .as_str(),
            "$"
        );
        assert!(JsonPath::root().parent().is_none());
    }
}
