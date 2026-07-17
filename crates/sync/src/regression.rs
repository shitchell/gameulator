//! Playtime regression check: a pure guard that flags when an incoming save's
//! playtime is behind the latest snapshot — the tell-tale of a stale device
//! clobbering a newer save.

/// Whether an incoming save's playtime is consistent with the latest snapshot,
/// or is a regression (a stale device overwrote a newer save).
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "a Regression result must be surfaced as an alarm, not dropped"]
pub enum RegressionCheck {
    /// Incoming playtime is >= the latest snapshot (or there is no snapshot yet).
    Accept,
    /// Incoming playtime is BEHIND the latest snapshot — the tell-tale of a stale
    /// device clobbering a newer save.
    Regression { incoming: u32, latest: u32 },
}

/// Compare an incoming save's total playtime (minutes) against the latest
/// snapshot's. Playtime is monotonic in normal play, so incoming < latest means
/// a stale save. Equal is accepted (a legitimate re-save). No latest snapshot
/// (first save) is accepted — nothing to compare against.
pub fn check(incoming_minutes: u32, latest_minutes: Option<u32>) -> RegressionCheck {
    match latest_minutes {
        Some(latest) if incoming_minutes < latest => RegressionCheck::Regression {
            incoming: incoming_minutes,
            latest,
        },
        _ => RegressionCheck::Accept,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progressed_playtime_is_accepted() {
        assert_eq!(check(1200, Some(1000)), RegressionCheck::Accept);
    }

    #[test]
    fn equal_playtime_is_accepted_as_a_resave() {
        assert_eq!(check(1000, Some(1000)), RegressionCheck::Accept);
    }

    #[test]
    fn behind_playtime_is_a_regression() {
        assert_eq!(
            check(900, Some(1000)),
            RegressionCheck::Regression {
                incoming: 900,
                latest: 1000
            }
        );
    }

    #[test]
    fn first_save_with_no_snapshot_is_accepted() {
        assert_eq!(check(500, None), RegressionCheck::Accept);
    }

    #[test]
    fn both_zero_is_accepted() {
        assert_eq!(check(0, Some(0)), RegressionCheck::Accept);
    }
}
