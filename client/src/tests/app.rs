use super::{auto_idle_change, IDLE_AFTER};
use std::time::Duration;

#[test]
fn going_idle_waits_for_the_limit_and_coming_back_is_immediate() {
    assert_eq!(auto_idle_change(true, Duration::from_secs(1), false), None);
    assert_eq!(auto_idle_change(true, IDLE_AFTER, false), Some(true));
    assert_eq!(auto_idle_change(true, Duration::from_secs(1), true), Some(false));
    assert_eq!(auto_idle_change(true, IDLE_AFTER, true), None);
}

#[test]
fn a_chosen_status_stops_going_idle_but_not_coming_back() {
    assert_eq!(auto_idle_change(false, IDLE_AFTER * 2, false), None);
    assert_eq!(auto_idle_change(false, Duration::ZERO, true), Some(false));
}
