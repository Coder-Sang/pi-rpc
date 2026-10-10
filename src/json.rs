//! Shared bounded, stack-protected JSON conversion. Wire framing lives in codec.
use crate::{Error, Result};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub(crate) const MAX_DEPTH: usize = 512;

fn depth_error() -> Error {
    Error::Protocol("JSON nesting depth limit exceeded (512 containers)".into())
}

/// Count containers before recursive parsing allocates any nested values.
/// This is a depth guard, not a substitute for serde_json's syntax validation.
pub(crate) fn check_wire_depth(bytes: &[u8]) -> Result<()> {
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > MAX_DEPTH {
                        return Err(depth_error());
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    Ok(())
}

/// Check caller-created Values without cloning or recursive traversal.
pub(crate) fn check_value_depth(value: &Value) -> Result<()> {
    // Iterators keep the traversal stack bounded by depth, not object width.
    let mut levels = vec![Box::new(std::iter::once(value)) as Box<dyn Iterator<Item = &Value>>];
    while let Some(level) = levels.last_mut() {
        let Some(value) = level.next() else {
            levels.pop();
            continue;
        };
        let children: Box<dyn Iterator<Item = &Value>> = match value {
            Value::Array(values) => Box::new(values.iter()),
            Value::Object(values) => Box::new(values.values()),
            _ => continue,
        };
        if levels.len() > MAX_DEPTH {
            return Err(depth_error());
        }
        levels.push(children);
    }
    Ok(())
}

pub(crate) fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    check_wire_depth(bytes)?;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    deserializer.disable_recursion_limit();
    let value = T::deserialize(serde_stacker::Deserializer::new(&mut deserializer))
        .map_err(|_| Error::Protocol("invalid JSON record".into()))?;
    deserializer
        .end()
        .map_err(|_| Error::Protocol("invalid JSON record".into()))?;
    Ok(value)
}

pub(crate) fn decode<T: DeserializeOwned>(value: &Value) -> Result<T> {
    check_value_depth(value)?;
    T::deserialize(serde_stacker::Deserializer::new(value))
        .map_err(|_| Error::Protocol("invalid JSON value".into()))
}

pub(crate) fn to_value<T: Serialize>(value: &T) -> Result<Value> {
    let value = value
        .serialize(serde_stacker::Serializer::new(
            serde_json::value::Serializer,
        ))
        .map_err(|_| Error::Protocol("cannot encode JSON".into()))?;
    if let Err(error) = check_value_depth(&value) {
        drop_value(value);
        return Err(error);
    }
    Ok(value)
}

/// Reject arbitrarily deep caller-created Values without recursive destruction.
pub(crate) fn drop_value(value: Value) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::Array(values) => pending.extend(values),
            Value::Object(values) => pending.extend(values.into_values()),
            _ => {}
        }
    }
}
