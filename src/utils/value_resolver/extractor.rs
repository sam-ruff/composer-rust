use super::traits::ReferenceExtractor;
use once_cell::sync::Lazy;
use regex::Regex;

/// Regex to match Jinja2 variable expressions and extract the variable name.
/// Matches: {{ variable }}, {{ var.nested }}, {{ var | filter }}, etc.
/// Captures only the variable name (group 1), ignoring filters.
static TEMPLATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)(?:\s*\|[^}]*)?\s*\}\}",
    )
    .expect("Invalid regex pattern")
});

/// Regex to check if string contains any template syntax
static HAS_TEMPLATE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{.*?\}\}").expect("Invalid regex pattern"));

/// Reference extractor implementation using regex to parse MiniJinja/Jinja2 syntax.
pub struct MiniJinjaReferenceExtractor;

impl MiniJinjaReferenceExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MiniJinjaReferenceExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferenceExtractor for MiniJinjaReferenceExtractor {
    fn extract_references(&self, template_str: &str) -> Vec<String> {
        TEMPLATE_REGEX
            .captures_iter(template_str)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    fn contains_template(&self, s: &str) -> bool {
        HAS_TEMPLATE_REGEX.is_match(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_reference() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("{{ foo }}");
        assert_eq!(refs, vec!["foo"]);
    }

    #[test]
    fn test_extract_nested_reference() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("{{ parent.child.grandchild }}");
        assert_eq!(refs, vec!["parent.child.grandchild"]);
    }

    #[test]
    fn test_extract_with_filter() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("{{ name | upper }}");
        assert_eq!(refs, vec!["name"]);
    }

    #[test]
    fn test_extract_with_complex_filter() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("{{ name | default('fallback') }}");
        assert_eq!(refs, vec!["name"]);
    }

    #[test]
    fn test_extract_multiple_references() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("{{ part1 }} and {{ part2 }}");
        assert_eq!(refs, vec!["part1", "part2"]);
    }

    #[test]
    fn test_extract_mixed_text_and_references() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("Hello {{ name }}, welcome to {{ place }}!");
        assert_eq!(refs, vec!["name", "place"]);
    }

    #[test]
    fn test_extract_no_references() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("Hello world!");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_extract_with_whitespace_variations() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("{{foo}} {{  bar  }} {{   baz   }}");
        assert_eq!(refs, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_contains_template_true() {
        let extractor = MiniJinjaReferenceExtractor::new();
        assert!(extractor.contains_template("Hello {{ world }}"));
    }

    #[test]
    fn test_contains_template_false() {
        let extractor = MiniJinjaReferenceExtractor::new();
        assert!(!extractor.contains_template("Hello world"));
    }

    #[test]
    fn test_contains_template_empty_braces() {
        let extractor = MiniJinjaReferenceExtractor::new();
        assert!(extractor.contains_template("Hello {{}} world"));
    }

    #[test]
    fn test_extract_underscore_in_name() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("{{ my_variable_name }}");
        assert_eq!(refs, vec!["my_variable_name"]);
    }

    #[test]
    fn test_extract_numbers_in_name() {
        let extractor = MiniJinjaReferenceExtractor::new();
        let refs = extractor.extract_references("{{ var1.item2 }}");
        assert_eq!(refs, vec!["var1.item2"]);
    }
}
