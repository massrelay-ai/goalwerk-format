//! The kernel's RRULE subset (massrelay lifecycle.rs parse_recurrence): FREQ=DAILY|WEEKLY|MONTHLY|YEARLY, INTERVAL, COUNT.
use atingo_path::rrule_ok;

#[test]
fn accepts_the_kernel_subset() {
    for r in ["FREQ=WEEKLY", "FREQ=WEEKLY;INTERVAL=2;COUNT=10", "RRULE:FREQ=DAILY", "FREQ=MONTHLY;COUNT=1", "FREQ=YEARLY;INTERVAL=366"] {
        assert!(rrule_ok(r), "{r} should be ok");
    }
}

#[test]
fn refuses_everything_else() {
    for r in ["FREQ=WEEKLY;BYDAY=MO", "FREQ=HOURLY", "FREQ=DAILY;INTERVAL=0", "INTERVAL=0", "INTERVAL=2", "FREQ=DAILY;COUNT=0", "FREQ=DAILY;INTERVAL=367", "FREQ=DAILY;UNTIL=20261231", "", "FREQ", "FREQ=DAILY;COUNT=x"] {
        assert!(!rrule_ok(r), "{r} should be refused");
    }
}
