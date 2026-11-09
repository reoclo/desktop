use reoclo_desktop::control::Client;
use reoclo_desktop::fake::FakeDaemon;
use reoclo_desktop::model::TunnelList;
use serde_json::json;
use std::time::Duration;

#[test]
fn socket_crud_and_chunked_events_share_the_real_wire_contract() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDaemon::start(dir.path().join("daemon.sock")).unwrap();
    let client = Client::new(fake.path());
    let mut events = client.events().unwrap();
    fake.wait_for_subscriber(Duration::from_secs(3)).unwrap();
    client
        .request::<serde_json::Value>("POST", "/v1/tunnels/grafana/down", None)
        .unwrap();
    let event = events.next_event().unwrap().unwrap();
    assert_eq!(event.name, "tunnel");
    assert!(event.data.contains("\"state\":\"down\""));
    let list: TunnelList = client.request("GET", "/v1/tunnels", None).unwrap();
    assert!(
        !list
            .tunnels
            .iter()
            .find(|t| t.name == "grafana")
            .unwrap()
            .enabled
    );
}

#[test]
fn errors_preserve_daemon_code_and_message() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDaemon::start(dir.path().join("daemon.sock")).unwrap();
    let error = Client::new(fake.path())
        .request::<serde_json::Value>("POST", "/v1/tunnels/missing/up", Some(&json!({})))
        .unwrap_err();
    assert!(error.to_string().contains("not_found"));
    assert!(error.to_string().contains("missing"));
}

#[test]
fn settings_patch_persists_and_accepts_the_daemons_full_port_range() {
    use reoclo_desktop::model::Settings;
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDaemon::start(dir.path().join("daemon.sock")).unwrap();
    let client = Client::new(fake.path());
    let changed: Settings = client
        .request(
            "PATCH",
            "/v1/settings",
            Some(&json!({"https_port":9443,"idle_timeout_seconds":120})),
        )
        .unwrap();
    assert_eq!(changed.https_port, 9443);
    let read: Settings = client.request("GET", "/v1/settings", None).unwrap();
    assert_eq!(read.idle_timeout_seconds, 120);
    assert_eq!(
        client
            .request::<Settings>("PATCH", "/v1/settings", Some(&json!({"https_port":443})))
            .unwrap()
            .https_port,
        443
    );
    assert!(
        client
            .request::<Settings>("PATCH", "/v1/settings", Some(&json!({"https_port":0})))
            .is_err()
    );
}

#[test]
fn fragmented_chunked_sse_handles_multiline_data_and_heartbeat() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
            .unwrap();
        for part in [
            ": ping\n\neve",
            "nt: auth\nda",
            "ta: first\ndata: second\n\n",
        ] {
            write!(stream, "{:x}\r\n{part}\r\n", part.len()).unwrap();
        }
        stream.write_all(b"0\r\n\r\n").unwrap();
    });
    let mut events = Client::new(path).events().unwrap();
    let event = events.next_event().unwrap().unwrap();
    assert_eq!(event.name, "auth");
    assert_eq!(event.data, "first\nsecond");
    assert!(events.next_event().unwrap().is_none());
    server.join().unwrap();
}
