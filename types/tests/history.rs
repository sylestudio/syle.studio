use syle_types::History;

#[test]
fn new_history_has_only_the_initial_present() {
    let h = History::new(1);
    assert_eq!(*h.present(), 1);
    assert!(!h.can_undo());
    assert!(!h.can_redo());
}

#[test]
fn record_advances_present_and_enables_undo() {
    let mut h = History::new(1);
    h.record(2);
    assert_eq!(*h.present(), 2);
    assert!(h.can_undo());
    assert!(!h.can_redo());
}

#[test]
fn undo_then_redo_round_trips() {
    let mut h = History::new(1);
    h.record(2);
    h.record(3);
    assert_eq!(h.undo().copied(), Some(2));
    assert_eq!(*h.present(), 2);
    assert!(h.can_redo());
    assert_eq!(h.undo().copied(), Some(1));
    assert_eq!(h.redo().copied(), Some(2));
    assert_eq!(h.redo().copied(), Some(3));
    assert!(!h.can_redo());
}

#[test]
fn recording_a_new_state_clears_redo() {
    let mut h = History::new(1);
    h.record(2);
    h.undo(); // present = 1, future = [2]
    assert!(h.can_redo());
    h.record(9); // a new branch discards the old redo
    assert_eq!(*h.present(), 9);
    assert!(!h.can_redo());
}

#[test]
fn recording_an_unchanged_state_is_a_noop_and_keeps_redo() {
    let mut h = History::new(1);
    h.record(2);
    h.undo(); // present = 1, future = [2]
    h.record(1); // equal to present → ignored
    assert_eq!(*h.present(), 1);
    assert!(h.can_redo()); // redo preserved — debounce-loop safety
    assert_eq!(h.redo().copied(), Some(2));
}

#[test]
fn undo_and_redo_at_the_ends_return_none() {
    let mut h = History::new(1);
    assert_eq!(h.undo(), None);
    assert_eq!(h.redo(), None);
}

#[test]
fn past_is_capped_at_the_limit() {
    let mut h = History::with_limit(0, 3);
    for n in 1..=10 {
        h.record(n);
    }
    assert_eq!(*h.present(), 10);
    // Only `limit` undo steps are retained; the oldest are dropped.
    let mut steps = 0;
    while h.undo().is_some() {
        steps += 1;
    }
    assert_eq!(steps, 3);
}
