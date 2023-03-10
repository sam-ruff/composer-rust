use serde_yaml::{Mapping, Value};

pub(crate) fn parse_yaml_string(yaml_str: &str) -> Value {
    let (key_path, value) = yaml_str.split_once("=").unwrap();
    let keys = key_path.split(".");

    let mut map = Mapping::new();
    let mut nested_map = &mut map;

    for key in keys.clone().take(keys.clone().count() - 1) {
        let new_map = Mapping::new();
        nested_map.insert(Value::from(key), Value::Mapping(new_map));
        nested_map = match nested_map.get_mut(key).unwrap() {
            Value::Mapping(map) => map,
            _ => unreachable!(),
        };
    }

    let last_key = keys.last().unwrap();
    nested_map.insert(
        Value::from(last_key.to_owned()),
        Value::String(value.to_owned()),
    );

    Value::Mapping(map)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::from_str;

    #[test]
    fn test_parse_yaml_string() -> anyhow::Result<()> {
        let actual = parse_yaml_string("foo.bar=baz");
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
        let actual = parse_yaml_string("foo=bar");
        let yaml = "
        foo: bar
        ";
        let expected: Value = from_str(yaml)?;
        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_parse_yaml_string_long() -> anyhow::Result<()> {
        let actual = parse_yaml_string("foo.bar.baz.terry=bar");
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
}
