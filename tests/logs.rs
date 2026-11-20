use reoclo_desktop::{logs::LogTail, reducer::Model};
use std::io::Write;

#[test]
fn log_events_retain_a_bounded_window_larger_than_fifty_lines() {
    let mut model = Model::default();
    for i in 0..10_010 {
        model
            .apply(
                "log",
                &serde_json::json!({"line":format!("tunnel db: {i}")}).to_string(),
            )
            .unwrap();
    }
    assert_eq!(model.recent_logs.len(), 10_000);
    assert_eq!(model.recent_logs.front().unwrap().line, "tunnel db: 10");
}

#[test]
fn file_tail_preserves_timestamps_and_waits_for_complete_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tunneld.log");
    std::fs::write(&path, "2026-11-20T12:00:00Z tunnel db: ready\npartial").unwrap();
    let mut tail = LogTail::new(path.clone());
    let entries = tail.read().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].line, "tunnel db: ready");
    assert_eq!(entries[0].received.unix_timestamp(), 1795176000);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    writeln!(file, " line").unwrap();
    assert_eq!(tail.read().unwrap()[0].line, "partial line");
    assert!(tail.read().unwrap().is_empty());
}

#[test]
fn file_tail_recovers_from_rotation_and_truncation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tunneld.log");
    std::fs::write(&path, "old line with a long message\n").unwrap();
    let mut tail = LogTail::new(path.clone());
    tail.read().unwrap();
    std::fs::write(&path, "short\n").unwrap();
    assert_eq!(tail.read().unwrap()[0].line, "short");
    std::fs::rename(&path, dir.path().join("previous.log")).unwrap();
    std::fs::write(&path, "rotated\n").unwrap();
    assert_eq!(tail.read().unwrap()[0].line, "rotated");
}

#[test]
fn log_filter_matches_whole_tunnel_names_and_case_insensitive_text() {
    let mut model = Model::default();
    for line in [
        "tunnel db: Ready",
        "pool db,web: failed",
        "tunnel db-copy: Ready",
    ] {
        model
            .apply("log", &serde_json::json!({"line":line}).to_string())
            .unwrap();
    }
    let rows: Vec<_> = model
        .recent_logs
        .iter()
        .filter(|e| e.matches("db", "READY"))
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].line, "tunnel db: Ready");
}

#[test]
fn file_and_event_copies_pair_once_without_losing_repeated_messages() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tunneld.log");
    std::fs::write(
        &path,
        "2026-11-20T12:00:00Z tunnel db: ready\n2026-11-20T12:00:00Z tunnel db: ready\n",
    )
    .unwrap();
    let at = time::OffsetDateTime::from_unix_timestamp(1795176000).unwrap();
    for disk_first in [true, false] {
        let mut model = Model::default();
        let disk = LogTail::new(path.clone()).read().unwrap();
        let events =
            [0, 1].map(|_| reoclo_desktop::logs::LogEntry::event("tunnel db: ready".into(), at));
        if disk_first {
            for entry in disk.into_iter().chain(events) {
                model.append_log(entry);
            }
        } else {
            for entry in events.into_iter().chain(disk) {
                model.append_log(entry);
            }
        }
        assert_eq!(model.recent_logs.len(), 2);
        assert_ne!(model.recent_logs[0].id, model.recent_logs[1].id);
    }
}
