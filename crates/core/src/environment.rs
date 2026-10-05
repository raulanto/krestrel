use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub variables: IndexMap<String, EnvVariable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVariable {
    pub value: String,
    pub enabled: bool,
    #[serde(default)]
    pub secret: bool,
}

impl Environment {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            variables: IndexMap::new(),
        }
    }

    pub fn with_variable(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        enabled: bool,
        secret: bool,
    ) -> Self {
        self.variables.insert(
            name.into(),
            EnvVariable {
                value: value.into(),
                enabled,
                secret,
            },
        );
        self
    }
}

/// Resolves placeholders in the format `{{variable_name}}` against a chain of environments.
/// Environments are evaluated from top to bottom (earlier in the slice takes precedence,
/// or base environment overridden by active environment).
/// Supports recursive variable resolution up to a max depth of 10 to avoid infinite loops.
pub fn resolve_variables(text: &str, environments: &[&Environment]) -> String {
    resolve_variables_with_depth(text, environments, 0)
}

fn resolve_variables_with_depth(text: &str, environments: &[&Environment], depth: usize) -> String {
    if depth > 10 || !text.contains("{{") {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'

            let mut var_name = String::new();
            let mut found_close = false;

            while let Some(c) = chars.next() {
                if c == '}' && chars.peek() == Some(&'}') {
                    chars.next(); // consume second '}'
                    found_close = true;
                    break;
                }
                var_name.push(c);
            }

            if found_close {
                let trimmed = var_name.trim();
                let mut replaced = None;

                for env in environments {
                    if let Some(var) = env.variables.get(trimmed) {
                        if !var.enabled {
                            continue;
                        }
                        // Recursively resolve any nested variables inside the value
                        let resolved_val =
                            resolve_variables_with_depth(&var.value, environments, depth + 1);
                        replaced = Some(resolved_val);
                        break;
                    }
                }

                if let Some(val) = replaced {
                    result.push_str(&val);
                } else {
                    // Keep placeholder intact if not found or disabled
                    result.push_str("{{");
                    result.push_str(&var_name);
                    result.push_str("}}");
                }
            } else {
                result.push_str("{{");
                result.push_str(&var_name);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_resolution_simple() {
        let mut env = Environment::new("env1", "Development");
        env = env.with_variable("baseUrl", "https://api.kestrel.dev", true, false);
        env = env.with_variable("apiVersion", "v1", true, false);

        let input = "{{baseUrl}}/{{apiVersion}}/users";
        let resolved = resolve_variables(input, &[&env]);
        assert_eq!(resolved, "https://api.kestrel.dev/v1/users");
    }

    #[test]
    fn test_variable_resolution_inheritance_and_override() {
        let mut base_env = Environment::new("base", "Global");
        base_env = base_env.with_variable("host", "example.com", true, false);
        base_env = base_env.with_variable("port", "80", true, false);

        let mut dev_env = Environment::new("dev", "Development");
        dev_env = dev_env.with_variable("port", "8080", true, false);

        let input = "http://{{host}}:{{port}}/api";
        // dev_env takes precedence over base_env
        let resolved = resolve_variables(input, &[&dev_env, &base_env]);
        assert_eq!(resolved, "http://example.com:8080/api");
    }

    #[test]
    fn test_variable_resolution_disabled() {
        let mut env = Environment::new("env1", "Development");
        env = env.with_variable("apiKey", "secret-token", false, true);

        let input = "Bearer {{apiKey}}";
        let resolved = resolve_variables(input, &[&env]);
        // Disabled variable should remain unresolved
        assert_eq!(resolved, "Bearer {{apiKey}}");
    }

    #[test]
    fn test_variable_resolution_nested() {
        let mut env = Environment::new("env1", "Development");
        env = env.with_variable("domain", "kestrel.local", true, false);
        env = env.with_variable("baseUrl", "https://{{domain}}/api", true, false);

        let input = "{{baseUrl}}/ping";
        let resolved = resolve_variables(input, &[&env]);
        assert_eq!(resolved, "https://kestrel.local/api/ping");
    }

    #[test]
    fn test_variable_resolution_infinite_loop_protection() {
        let mut env = Environment::new("env1", "Development");
        env = env.with_variable("a", "{{b}}", true, false);
        env = env.with_variable("b", "{{a}}", true, false);

        let input = "{{a}}";
        let _ = resolve_variables(input, &[&env]); // Must not panic or hang
    }
}
