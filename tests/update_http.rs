use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use reoclo_desktop::update::{Client, Error, Manifest};
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    time::Duration,
};

#[test]
fn client_fetches_latest_then_verifies_the_archive_from_local_http() {
    for corrupted in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let mut seed = [0; 32];
        std::fs::File::open("/dev/urandom")
            .unwrap()
            .read_exact(&mut seed)
            .unwrap();
        let key = SigningKey::from_bytes(&seed);
        let bytes = b"a signed archive".to_vec();
        let manifest = Manifest {
            version: "0.6.0".parse().unwrap(),
            pub_date: "2026-11-20T00:00:00Z".into(),
            notes: "Test release".into(),
            url: format!("http://{address}/app.tar.gz"),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            signature: STANDARD.encode(key.sign(&bytes).to_bytes()),
        };
        let server = std::thread::spawn(move || {
            let body = if corrupted {
                b"corrupt".to_vec()
            } else {
                bytes
            };
            for (path, body) in [
                ("/latest.json", serde_json::to_vec(&manifest).unwrap()),
                ("/app.tar.gz", body),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = String::new();
                BufReader::new(&stream).read_line(&mut request).unwrap();
                assert!(request.starts_with(&format!("GET {path} ")));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        let client =
            Client::new(format!("http://{address}/latest.json"), key.verifying_key()).unwrap();
        let release = client.check("0.5.0").unwrap().unwrap();
        let result = client.download(release);
        if corrupted {
            assert!(matches!(result, Err(Error::Checksum)));
        } else {
            assert_eq!(result.unwrap().manifest.version.to_string(), "0.6.0");
        }
        server.join().unwrap();
    }
}
