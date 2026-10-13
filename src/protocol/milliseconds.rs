//! Number-compatible wire durations behind the existing integer millisecond API.
use serde::{
    Deserialize, Deserializer,
    de::{Error, Unexpected, Visitor},
};
use std::fmt;

pub(super) fn deserialize_milliseconds<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    struct MillisecondsVisitor;
    impl Visitor<'_> for MillisecondsVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("finite nonnegative milliseconds smaller than 2^64")
        }

        fn visit_u64<E: Error>(self, value: u64) -> Result<u64, E> {
            // Do not route integers through f64: that would lose precision above 2^53.
            Ok(value)
        }

        fn visit_i64<E: Error>(self, value: i64) -> Result<u64, E> {
            u64::try_from(value).map_err(|_| E::invalid_value(Unexpected::Signed(value), &self))
        }

        fn visit_f64<E: Error>(self, value: f64) -> Result<u64, E> {
            // u64::MAX rounds UP to 2^64 as f64; equality must be rejected before casting.
            if value.is_finite() && (0.0..18_446_744_073_709_551_616.0).contains(&value) {
                Ok(value as u64)
            } else {
                Err(E::invalid_value(Unexpected::Float(value), &self))
            }
        }
    }
    deserializer.deserialize_any(MillisecondsVisitor)
}

pub(super) fn deserialize_optional_milliseconds<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Milliseconds(u64);
    impl<'de> Deserialize<'de> for Milliseconds {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserialize_milliseconds(deserializer).map(Self)
        }
    }
    Option::<Milliseconds>::deserialize(deserializer).map(|value| value.map(|value| value.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::value::{Error, F64Deserializer};

    #[test]
    fn nonfinite_and_out_of_range_durations_are_not_saturating_casts() {
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -0.25,
            u64::MAX as f64,
        ] {
            assert!(deserialize_milliseconds(F64Deserializer::<Error>::new(value)).is_err());
            assert!(
                deserialize_optional_milliseconds(F64Deserializer::<Error>::new(value)).is_err()
            );
        }
    }
}
