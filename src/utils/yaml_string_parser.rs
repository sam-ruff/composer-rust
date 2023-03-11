use anyhow::anyhow;
use serde_yaml::{Mapping, Value};
use std::borrow::Cow;

/// Parses a string in the format "x.y.z=foo" into its appropriate YAML representation.
///
/// This function takes a string in the format of a dot-separated key path, followed by an equals sign and a value.
/// It then creates a YAML mapping with nested mappings for each key in the path, with the final key-value pair being
/// added as a string value to the nested mapping.
///
/// # Errors
///
/// This function returns an `anyhow::Error` if the input string is not in the expected format of "x.y.z=foo" or if
/// the key path is empty.
///
/// # Examples
///
/// ```
/// use serde_yaml::Value;
/// use anyhow::Result;
///
/// fn main() -> Result<()> {
///     let yaml_str = "abc.bcd.dge=xyz";
///     let yaml_value = parse_yaml_string(yaml_str)?;
///
///     assert_eq!(yaml_value["abc"]["bcd"]["dge"], Value::String("xyz".to_owned()));
///
///     Ok(())
/// }
/// ```
///
/// # Arguments
///
/// * `yaml_str` - A string in the format of "x.y.z=foo", where the key path is separated by dots and
///                followed by an equals sign and a value.
///
/// # Returns
///
/// A `serde_yaml::Value` object representing the YAML mapping created from the input string.
///
/// # Panics
///
/// This function may panic if there is an internal error when accessing nested mappings.
pub(crate) fn parse_yaml_string(yaml_str: &str) -> anyhow::Result<Value> {
    let (key_path, value) = yaml_str.split_once("=").ok_or_else(|| {
        anyhow!(
            "Failed to split YAML string: {}, must be the format x.y.z=foo",
            yaml_str
        )
    })?;

    if key_path.is_empty() {
        return Err(anyhow!(
            "Failed to find yaml key for string {}, must be the format x.y.z=foo.",
            yaml_str
        ));
    }

    let keys = key_path.split(".");
    let mut map = Mapping::new();
    let mut nested_map = &mut map;

    for key in keys.clone().take(keys.clone().count().saturating_sub(1)) {
        let new_map = Mapping::new();
        nested_map.insert(Value::from(Cow::Borrowed(key)), Value::Mapping(new_map));
        nested_map = match nested_map.get_mut(key).unwrap() {
            Value::Mapping(map) => map,
            _ => unreachable!(),
        };
    }

    let last_key = keys.last().unwrap();
    nested_map.insert(
        Value::from(Cow::Borrowed(last_key)),
        Value::String(value.to_owned()),
    );

    Ok(Value::Mapping(map))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::from_str;

    #[test]
    fn test_parse_yaml_string() -> anyhow::Result<()> {
        let actual = parse_yaml_string("foo.bar=baz")?;
        let yaml = "
        foo:
            bar: 'baz'
        ";
        let expected: Value = from_str(yaml)?;
        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_parse_yaml_string_simple() -> anyhow::Result<()> {
        let actual = parse_yaml_string("foo=bar")?;
        let yaml = "
        foo: bar
        ";
        let expected: Value = from_str(yaml)?;
        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_parse_yaml_string_long() -> anyhow::Result<()> {
        let actual = parse_yaml_string("foo.bar.baz.terry=bar")?;
        let yaml = "
        foo:
            bar:
                baz:
                   terry: bar
        ";
        let expected: Value = from_str(yaml)?;
        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_parse_yaml_invalid_string() -> anyhow::Result<()> {
        let err = parse_yaml_string("invalid").unwrap_err();
        let actual_err = err.to_string();
        let expected_err =
            "Failed to split YAML string: invalid, must be the format x.y.z=foo".to_string();
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    fn test_parse_yaml_missing_key() -> anyhow::Result<()> {
        let err = parse_yaml_string("=hello").unwrap_err();
        let actual_err = err.to_string();
        let expected_err =
            "Failed to find yaml key for string =hello, must be the format x.y.z=foo.".to_string();
        assert_eq!(expected_err, actual_err);
        Ok(())
    }
}
