use anyhow::Result;
use serde_yaml::Value;

#[cfg(test)]
use mockall::automock;

/// Trait for extracting template references from string values.
/// This abstraction allows mocking the extraction logic in tests.
#[cfg_attr(test, automock)]
pub trait ReferenceExtractor {
    /// Extracts variable references from a template string.
    /// E.g., "{{ foo.bar }}" returns vec!["foo.bar"]
    /// E.g., "{{ name | upper }}" returns vec!["name"]
    fn extract_references(&self, template_str: &str) -> Vec<String>;

    /// Checks if a string contains template syntax
    fn contains_template(&self, s: &str) -> bool;
}

/// Trait for rendering template strings with provided values.
/// Wraps MiniJinja to allow mocking in tests.
#[cfg_attr(test, automock)]
pub trait TemplateRenderer {
    /// Renders a template string with the given context values.
    fn render(&self, template_str: &str, context: &Value) -> Result<String>;
}
