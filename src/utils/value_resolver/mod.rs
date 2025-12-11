mod dependency_graph;
mod extractor;
pub mod traits;

use anyhow::{anyhow, Context, Result};
use minijinja::Environment;
use serde_yaml::Value;
use std::collections::HashMap;

use dependency_graph::{DependencyGraph, ValuePath};
use extractor::MiniJinjaReferenceExtractor;
use traits::{ReferenceExtractor, TemplateRenderer};

/// Production implementation of TemplateRenderer using MiniJinja
pub struct MiniJinjaRenderer;

impl MiniJinjaRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MiniJinjaRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRenderer for MiniJinjaRenderer {
    fn render(&self, template_str: &str, context: &Value) -> Result<String> {
        let mut env = Environment::new();

        env.add_template("inline", template_str)
            .with_context(|| format!("Failed to parse template: {}", template_str))?;

        let template = env.get_template("inline")?;
        let ctx = minijinja::value::Value::from_serializable(context);

        template
            .render(&ctx)
            .map_err(|e| anyhow!("Failed to render value reference '{}': {}", template_str, e))
    }
}

/// Resolves all value references in the given YAML structure using default implementations.
/// This is the main public entry point for value resolution.
pub fn resolve_value_references(values: Value) -> Result<Value> {
    let extractor = MiniJinjaReferenceExtractor::new();
    let renderer = MiniJinjaRenderer::new();
    resolve_with(values, &extractor, &renderer)
}

/// Resolves all value references using provided extractor and renderer.
/// Uses `&impl Trait` syntax for testability with mock implementations.
pub fn resolve_with(
    mut values: Value,
    extractor: &impl ReferenceExtractor,
    renderer: &impl TemplateRenderer,
) -> Result<Value> {
    // Step 1: Collect all template values (string values containing {{ }})
    let mut templates = HashMap::new();
    collect_template_values(&values, "", &mut templates, extractor);

    if templates.is_empty() {
        return Ok(values);
    }

    // Step 2: Build dependency graph
    let graph = build_dependency_graph(&templates, extractor);

    // Step 3: Topological sort (detects cycles)
    let resolution_order = graph.topological_sort()?;

    // Step 4: Resolve in order
    for path in resolution_order {
        if let Some(template_str) = templates.get(path.as_str()) {
            let rendered = renderer.render(template_str, &values)?;
            set_value_at_path(&mut values, path.as_str(), Value::String(rendered))?;
        }
    }

    Ok(values)
}

/// Recursively collects all value paths and their template strings
fn collect_template_values(
    value: &Value,
    current_path: &str,
    templates: &mut HashMap<String, String>,
    extractor: &impl ReferenceExtractor,
) {
    match value {
        Value::String(s) if extractor.contains_template(s) => {
            templates.insert(current_path.to_string(), s.clone());
        }
        Value::Mapping(map) => {
            for (key, val) in map {
                if let Value::String(key_str) = key {
                    let new_path = if current_path.is_empty() {
                        key_str.clone()
                    } else {
                        format!("{}.{}", current_path, key_str)
                    };
                    collect_template_values(val, &new_path, templates, extractor);
                }
            }
        }
        Value::Sequence(seq) => {
            for (idx, val) in seq.iter().enumerate() {
                let new_path = format!("{}[{}]", current_path, idx);
                collect_template_values(val, &new_path, templates, extractor);
            }
        }
        _ => {}
    }
}

/// Builds the dependency graph from template values
fn build_dependency_graph(
    templates: &HashMap<String, String>,
    extractor: &impl ReferenceExtractor,
) -> DependencyGraph {
    let mut graph = DependencyGraph::new();

    for (path, template_str) in templates {
        let from = ValuePath::new(path);
        graph.add_node(&from);

        let refs = extractor.extract_references(template_str);
        for ref_path in refs {
            let to = ValuePath::new(&ref_path);
            graph.add_dependency(&from, &to);
        }
    }

    graph
}

/// Sets a value at a given path (supports nested paths like "a.b.c")
fn set_value_at_path(value: &mut Value, path: &str, new_val: Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return Err(anyhow!("Empty path"));
    }

    if parts.len() == 1 {
        if let Value::Mapping(map) = value {
            map.insert(Value::String(parts[0].to_string()), new_val);
            return Ok(());
        }
        return Err(anyhow!("Cannot set value at path '{}': not a mapping", path));
    }

    let mut current = value;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - set the value
            if let Value::Mapping(map) = current {
                map.insert(Value::String(part.to_string()), new_val);
                return Ok(());
            }
            return Err(anyhow!(
                "Cannot set value at path '{}': parent is not a mapping",
                path
            ));
        } else {
            // Navigate deeper
            if let Value::Mapping(map) = current {
                current = map
                    .get_mut(Value::String(part.to_string()))
                    .ok_or_else(|| anyhow!("Path not found: {}", path))?;
            } else {
                return Err(anyhow!("Cannot navigate path '{}': not a mapping", path));
            }
        }
    }

    Err(anyhow!("Failed to set value at path: {}", path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::from_str;

    #[test]
    fn test_simple_reference_resolution() {
        let yaml = r#"
greeting: "hello"
message: "{{ greeting }} world"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("message").unwrap(),
            &Value::String("hello world".to_string())
        );
    }

    #[test]
    fn test_nested_value_reference() {
        let yaml = r#"
parent:
  child:
    value: "nested"
result: "{{ parent.child.value }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("result").unwrap(),
            &Value::String("nested".to_string())
        );
    }

    #[test]
    fn test_chained_references() {
        let yaml = r#"
a: "base"
b: "{{ a }}-extended"
c: "{{ b }}-final"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("c").unwrap(),
            &Value::String("base-extended-final".to_string())
        );
    }

    #[test]
    fn test_filter_support_upper() {
        let yaml = r#"
name: "world"
greeting: "{{ name | upper }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("greeting").unwrap(),
            &Value::String("WORLD".to_string())
        );
    }

    #[test]
    fn test_filter_support_lower() {
        let yaml = r#"
name: "HELLO"
greeting: "{{ name | lower }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("greeting").unwrap(),
            &Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_default_filter() {
        let yaml = r#"
message: "{{ undefined_var | default('fallback') }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("message").unwrap(),
            &Value::String("fallback".to_string())
        );
    }

    #[test]
    fn test_circular_dependency_error() {
        let yaml = r#"
a: "{{ b }}"
b: "{{ c }}"
c: "{{ a }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let result = resolve_value_references(values);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Circular dependency"),
            "Error should mention circular dependency: {}",
            err
        );
    }

    #[test]
    fn test_self_reference_error() {
        let yaml = r#"
a: "{{ a }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let result = resolve_value_references(values);

        assert!(result.is_err());
    }

    #[test]
    fn test_non_string_values_unchanged() {
        let yaml = r#"
number: 42
boolean: true
list:
  - item1
  - item2
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values.clone()).unwrap();

        assert_eq!(
            resolved.get("number").unwrap(),
            values.get("number").unwrap()
        );
        assert_eq!(
            resolved.get("boolean").unwrap(),
            values.get("boolean").unwrap()
        );
        assert_eq!(resolved.get("list").unwrap(), values.get("list").unwrap());
    }

    #[test]
    fn test_multiple_references_in_one_value() {
        let yaml = r#"
part1: "world"
part2: "hello"
message: "{{ part2 }} {{ part1 }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("message").unwrap(),
            &Value::String("hello world".to_string())
        );
    }

    #[test]
    fn test_no_templates_passthrough() {
        let yaml = r#"
simple: "value"
another: "plain text"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values.clone()).unwrap();

        assert_eq!(resolved, values);
    }

    #[test]
    fn test_nested_target_value() {
        let yaml = r#"
source: "hello"
target:
  nested:
    value: "{{ source }} world"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        let target = resolved.get("target").unwrap();
        let nested = target.get("nested").unwrap();
        let value = nested.get("value").unwrap();

        assert_eq!(value, &Value::String("hello world".to_string()));
    }

    #[test]
    fn test_deeply_nested_reference() {
        let yaml = r#"
level1:
  level2:
    level3:
      source: "deep"
result: "{{ level1.level2.level3.source }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("result").unwrap(),
            &Value::String("deep".to_string())
        );
    }

    #[test]
    fn test_mixed_static_and_dynamic_text() {
        let yaml = r#"
name: "Alice"
greeting: "Hello, {{ name }}! Welcome to the system."
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("greeting").unwrap(),
            &Value::String("Hello, Alice! Welcome to the system.".to_string())
        );
    }

    #[test]
    fn test_complex_dependency_chain() {
        let yaml = r#"
base: "foundation"
layer1: "{{ base }}-layer1"
layer2: "{{ layer1 }}-layer2"
final: "{{ layer2 }}-complete"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("final").unwrap(),
            &Value::String("foundation-layer1-layer2-complete".to_string())
        );
    }

    #[test]
    fn test_diamond_dependency() {
        // Both branch1 and branch2 depend on root
        // final depends on both branches
        let yaml = r#"
root: "base"
branch1: "{{ root }}-b1"
branch2: "{{ root }}-b2"
final: "{{ branch1 }} and {{ branch2 }}"
"#;
        let values: Value = from_str(yaml).unwrap();
        let resolved = resolve_value_references(values).unwrap();

        assert_eq!(
            resolved.get("final").unwrap(),
            &Value::String("base-b1 and base-b2".to_string())
        );
    }

    // Tests with mocked dependencies
    #[cfg(test)]
    mod mock_tests {
        use super::*;
        use crate::utils::value_resolver::traits::{MockReferenceExtractor, MockTemplateRenderer};

        #[test]
        fn test_resolver_with_mock_extractor() {
            let mut mock_extractor = MockReferenceExtractor::new();
            mock_extractor
                .expect_contains_template()
                .returning(|s| s.contains("{{"));
            mock_extractor
                .expect_extract_references()
                .returning(|_| vec!["foo".to_string()]);

            let mut mock_renderer = MockTemplateRenderer::new();
            mock_renderer
                .expect_render()
                .returning(|_, _| Ok("rendered".to_string()));

            let yaml = r#"
foo: "value"
bar: "{{ foo }}"
"#;
            let values: Value = from_str(yaml).unwrap();
            let result = resolve_with(values, &mock_extractor, &mock_renderer);

            assert!(result.is_ok());
        }

        #[test]
        fn test_renderer_error_propagates() {
            let mut mock_extractor = MockReferenceExtractor::new();
            mock_extractor
                .expect_contains_template()
                .returning(|s| s.contains("{{"));
            mock_extractor
                .expect_extract_references()
                .returning(|_| vec![]);

            let mut mock_renderer = MockTemplateRenderer::new();
            mock_renderer
                .expect_render()
                .returning(|_, _| Err(anyhow!("Render error")));

            let yaml = r#"
bar: "{{ foo }}"
"#;
            let values: Value = from_str(yaml).unwrap();
            let result = resolve_with(values, &mock_extractor, &mock_renderer);

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Render error"));
        }
    }
}
