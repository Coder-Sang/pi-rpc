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
    parse(value).is_some_and(|version| ([1, 3, 0]..=[1, 5, 0]).contains(&version))
}

pub(crate) fn minor(value: &str) -> Option<u64> {
    is_supported(value).then(|| parse(value).unwrap()[1])
}

pub(crate) fn enabled(value: &str) -> crate::Result<()> {
    let minor = minor(value).ok_or(crate::Error::Version)?;
    if !cfg!(any(
        feature = "pi_1_3",
        feature = "pi_1_4",
        feature = "pi_1_5"
    )) {
        return Err(crate::Error::NoProtocolEnabled);
    }
    let (enabled, feature) = match minor {
        3 => (cfg!(feature = "pi_1_3"), "pi_1_3"),
        4 => (cfg!(feature = "pi_1_4"), "pi_1_4"),
        5 => (cfg!(feature = "pi_1_5"), "pi_1_5"),
        _ => unreachable!("supported minor"),
    };
    if enabled {
        Ok(())
    } else {
        Err(crate::Error::VersionNotEnabled {
            version: value.into(),
            feature,
        })
    }
}

#[cfg(feature = "subprocess")]
pub(crate) enum Expectation<'a> {
    Supported,
    Through([u64; 3]),
    Exact(&'a str),
}

#[cfg(feature = "subprocess")]
impl<'a> Expectation<'a> {
    pub(crate) fn new(value: &'a str) -> crate::Result<Self> {
        if value == crate::PI_VERSION_REQUIREMENT {
            Ok(Self::Supported)
        } else if value == ">=1.1.0, <=1.3.0" {
            Ok(Self::Through([1, 3, 0]))
        } else if value == ">=1.1.0, <=1.4.0" {
            Ok(Self::Through([1, 4, 0]))
        } else if is_supported(value) {
            Ok(Self::Exact(value))
        } else {
            Err(crate::Error::Version)
        }
    }

    pub(crate) fn matches(&self, actual: &str) -> bool {
        is_supported(actual)
            && match self {
                Self::Supported => true,
                Self::Through(upper) => parse(actual).is_some_and(|v| v <= *upper),
                Self::Exact(expected) => actual == *expected,
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusive_range_uses_numeric_components() {
        for value in [
            "1.3.0",
            "1.3.1",
            "1.3.10",
            "1.3.18446744073709551615",
            "1.4.0",
            "1.4.1",
            "1.4.10",
            "1.4.18446744073709551615",
            "1.5.0",
        ] {
            assert!(is_supported(value), "{value}");
        }
        for value in ["0.99.2", "1.1.0", "1.2.999", "1.5.1", "1.10.0", "2.0.0"] {
            assert!(!is_supported(value), "{value}");
        }
    }

    #[test]
    fn only_canonical_bounded_three_component_releases_are_accepted() {
        for value in [
            "",
            "1",
            "1.3",
            "1.3.0.0",
            "01.3.0",
            "1.03.0",
            "1.3.00",
            "+1.3.0",
            "1.3.+0",
            "1.3.-1",
            "v1.3.0",
            " 1.3.0",
            "1.3.0\n",
            "1.3.0\0",
            "1.3.0-rc.1",
            "1.5.0+build",
            "1.3.18446744073709551616",
            "１.3.0",
            crate::PI_VERSION_REQUIREMENT,
            ">=1.1.0, <=1.2.0",
            ">=1.1.0, <=1.3.0",
            ">=1.1.0, <=1.4.0",
        ] {
            assert!(!is_supported(value), "{value:?}");
        }
        assert!(!is_supported(&format!("1.3.{}", "9".repeat(100_000))));
    }

    #[cfg(feature = "subprocess")]
    #[test]
    fn default_range_and_exact_locks_admit_supported_releases() {
        let range = Expectation::new(crate::PI_VERSION_REQUIREMENT).unwrap();
        for version in [
            "1.3.0", "1.3.1", "1.3.10", "1.4.0", "1.4.1", "1.4.10", "1.5.0",
        ] {
            assert!(range.matches(version));
            let exact = Expectation::new(version).unwrap();
            assert!(exact.matches(version));
            assert!(!exact.matches("1.3.9"));
        }
        for version in ["1.2.10", "1.5.1", "1.5.0-rc.1", "1.5.0+build"] {
            assert!(!range.matches(version));
            assert!(Expectation::new(version).is_err());
        }
    }

    #[cfg(feature = "subprocess")]
    #[test]
    fn exact_lock_patch_prefix_collisions_are_rejected() {
        for (shorter, longer) in [("1.3.1", "1.3.10"), ("1.4.1", "1.4.10")] {
            assert!(longer.starts_with(shorter));
            assert!(!shorter.starts_with(longer));
            for (expected, actual) in [(shorter, longer), (longer, shorter)] {
                assert!(is_supported(expected));
                assert!(is_supported(actual));
                let exact = Expectation::new(expected).unwrap();
                assert!(exact.matches(expected));
                assert!(
                    !exact.matches(actual),
                    "expected={expected} actual={actual}"
                );
            }
        }
    }

    #[cfg(feature = "subprocess")]
    #[test]
    fn legacy_ranges_keep_their_original_upper_bounds() {
        assert!(Expectation::new(">=1.1.0, <=1.2.0").is_err());
        let through_1_3 = Expectation::new(">=1.1.0, <=1.3.0").unwrap();
        let through_1_4 = Expectation::new(">=1.1.0, <=1.4.0").unwrap();
        for (actual, admitted_1_3, admitted_1_4) in [
            ("1.0.999", false, false),
            ("1.1.0", false, false),
            ("1.2.10", false, false),
            ("1.3.0", true, true),
            ("1.3.1", false, true),
            ("1.3.10", false, true),
            ("1.4.0", false, true),
            ("1.4.1", false, false),
            ("1.4.10", false, false),
            ("1.5.0", false, false),
            ("1.10.0", false, false),
            ("1.3.0-rc.1", false, false),
            ("1.3.0+build", false, false),
            (" 1.3.0", false, false),
            ("1.3.0\n", false, false),
        ] {
            assert_eq!(through_1_3.matches(actual), admitted_1_3, "{actual}");
            assert_eq!(through_1_4.matches(actual), admitted_1_4, "{actual}");
        }
    }

    #[cfg(feature = "subprocess")]
    #[test]
    fn other_constraint_expressions_are_rejected() {
        for expression in [
            "*",
            "^1.3.0",
            ">=1.3.0",
            ">=1.1.0,<=1.3.0",
            ">=1.1.0,<=1.4.0",
            ">=1.3.0,<=1.5.0",
            ">=1.2.0, <=1.5.0",
            "",
        ] {
            assert!(Expectation::new(expression).is_err(), "{expression}");
        }
    }
}
