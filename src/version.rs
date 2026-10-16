//! Shared version admission. This is deliberately not a general constraint parser.

fn parse(value: &str) -> Option<[u64; 3]> {
    // Three u64 components plus two dots. Reject oversized declarations early.
    if value.len() > 62 {
        return None;
    }
    let mut parts = value.split('.');
    let mut version = [0; 3];
    for component in &mut version {
        let part = parts.next()?;
        if part.is_empty()
            || (part.len() > 1 && part.starts_with('0'))
            || !part.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        *component = part.parse().ok()?;
    }
    parts.next().is_none().then_some(version)
}

pub(crate) fn is_supported(value: &str) -> bool {
    parse(value).is_some_and(|version| ([1, 1, 0]..=[1, 2, 0]).contains(&version))
}

#[cfg(feature = "subprocess")]
pub(crate) struct Expectation<'a>(Option<&'a str>);

#[cfg(feature = "subprocess")]
impl<'a> Expectation<'a> {
    pub(crate) fn new(value: &'a str) -> crate::Result<Self> {
        if value == crate::PI_VERSION_REQUIREMENT {
            Ok(Self(None))
        } else if is_supported(value) {
            Ok(Self(Some(value)))
        } else {
            Err(crate::Error::Version)
        }
    }

    pub(crate) fn matches(&self, actual: &str) -> bool {
        is_supported(actual) && self.0.is_none_or(|expected| actual == expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusive_range_uses_numeric_components() {
        for value in [
            "1.1.0",
            "1.1.1",
            "1.1.10",
            "1.1.18446744073709551615",
            "1.2.0",
        ] {
            assert!(is_supported(value), "{value}");
        }
        for value in ["0.99.2", "1.0.999", "1.2.1", "1.10.0", "2.0.0"] {
            assert!(!is_supported(value), "{value}");
        }
    }

    #[test]
    fn only_canonical_bounded_three_component_releases_are_accepted() {
        for value in [
            "",
            "1",
            "1.1",
            "1.1.0.0",
            "01.1.0",
            "1.01.0",
            "1.1.00",
            "+1.1.0",
            "1.1.+0",
            "1.1.-1",
            "v1.1.0",
            " 1.1.0",
            "1.1.0\n",
            "1.1.0\0",
            "1.1.0-rc.1",
            "1.2.0+build",
            "1.1.18446744073709551616",
            "１.1.0",
            crate::PI_VERSION_REQUIREMENT,
        ] {
            assert!(!is_supported(value), "{value:?}");
        }
        assert!(!is_supported(&format!("1.1.{}", "9".repeat(100_000))));
    }

    #[cfg(feature = "subprocess")]
    #[test]
    fn expectations_preserve_exact_locks_and_only_accept_the_fixed_range() {
        let range = Expectation::new(crate::PI_VERSION_REQUIREMENT).unwrap();
        for version in ["1.1.0", "1.1.1", "1.1.10", "1.2.0"] {
            assert!(range.matches(version));
            let exact = Expectation::new(version).unwrap();
            assert!(exact.matches(version));
            assert!(!exact.matches("1.1.9"));
        }
        for version in ["1.0.4", "1.2.1", "1.2.0-rc.1", "1.2.0+build"] {
            assert!(!range.matches(version));
            assert!(Expectation::new(version).is_err());
        }
        for expression in ["*", "^1.1.0", ">=1.1.0", ">=1.1.0,<=1.2.0", ""] {
            assert!(Expectation::new(expression).is_err());
        }
    }
}
