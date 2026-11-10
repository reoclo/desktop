use reoclo_desktop::link::{LinkError, LinkKind, is_open_link, parse};

const BASE: &str = "reoclo://open?v=1&org=acme&server=prod-1&port=5432&kind=tcp";

#[test]
fn a_full_link_parses_every_field() {
    let link = parse(
        "reoclo://open?v=1&org=acme&server=prod-1&port=3000&kind=https&host=db.internal&name=grafana-web",
    )
    .unwrap();
    assert_eq!(link.org, "acme");
    assert_eq!(link.server, "prod-1");
    assert_eq!(link.port, 3000);
    assert_eq!(link.kind, LinkKind::Https);
    assert_eq!(link.host, "db.internal");
    assert_eq!(link.name, "grafana-web");
    assert_eq!(link.target(), "db.internal:3000");
}

#[test]
fn optional_values_default() {
    let link = parse(BASE).unwrap();
    assert_eq!(link.host, "127.0.0.1");
    assert_eq!(link.name, "prod-1-5432");
    assert_eq!(link.kind, LinkKind::Tcp);
    assert_eq!(link.target(), "127.0.0.1:5432");
    let empty = parse(&format!("{BASE}&host=&name=")).unwrap();
    assert_eq!(empty, link);
}

#[test]
fn a_derived_name_follows_the_name_rule() {
    let link = parse("reoclo://open?v=1&org=acme&server=Srv_9&port=80&kind=https").unwrap();
    assert_eq!(link.name, "srv-9-80");
    let long = "a".repeat(64);
    let link = parse(&format!(
        "reoclo://open?v=1&org=acme&server={long}&port=80&kind=https"
    ))
    .unwrap();
    assert_eq!(link.name.len(), 63);
    assert!(link.name.ends_with("-80"));
}

#[test]
fn unknown_parameters_order_and_fragments_are_ignored() {
    let link =
        parse("reoclo://open?kind=tcp&utm=x&port=5432&server=prod-1&org=acme&v=1&future=%FF#top")
            .unwrap();
    assert_eq!(link.port, 5432);
    assert!(parse("reoclo://open/?v=1&org=acme&server=s&port=1&kind=tcp").is_ok());
    assert!(parse("REOCLO://open?v=1&org=acme&server=s&port=1&kind=tcp").is_ok());
}

#[test]
fn values_are_url_decoded() {
    let link = parse(
        "reoclo://open?v=1&org=acme&server=prod%2D1&port=5432&kind=tcp&host=db%2Eint&name=a%2Db",
    )
    .unwrap();
    assert_eq!(link.server, "prod-1");
    assert_eq!(link.host, "db.int");
    assert_eq!(link.name, "a-b");
}

#[test]
fn hosts_may_be_names_or_addresses() {
    let host = |value: &str| parse(&format!("{BASE}&host={value}")).map(|link| link.host);
    assert_eq!(host("10.0.0.7"), Ok("10.0.0.7".into()));
    assert_eq!(host("LocalHost"), Ok("localhost".into()));
    assert_eq!(host("%5B%3A%3A1%5D"), Ok("::1".into()));
    assert_eq!(host("fe80%3A%3A1"), Ok("fe80::1".into()));
    assert_eq!(
        parse(&format!("{BASE}&host=%5B%3A%3A1%5D"))
            .unwrap()
            .target(),
        "[::1]:5432"
    );
    for bad in [
        "bad%20host",
        "-x.com",
        "x-.com",
        "a..b",
        "a_b.com",
        "999.1.1.1.",
        "host%2Fpath",
        "http%3A%2F%2Fx",
    ] {
        assert_eq!(host(bad), Err(LinkError::Invalid("host")), "{bad}");
    }
}

#[test]
fn every_invalid_link_is_refused() {
    let with = |query: &str| parse(&format!("reoclo://open?{query}"));
    assert_eq!(parse("reoclo://tunnels"), Err(LinkError::NotALink));
    assert_eq!(parse("https://example.com"), Err(LinkError::NotALink));
    assert_eq!(parse("reoclo://openx?v=1"), Err(LinkError::NotALink));
    assert_eq!(
        with("org=a&server=s&port=1&kind=tcp"),
        Err(LinkError::Missing("version"))
    );
    assert_eq!(
        with("v=2&org=a&server=s&port=1&kind=tcp"),
        Err(LinkError::Version)
    );
    assert_eq!(
        with("v=1&server=s&port=1&kind=tcp"),
        Err(LinkError::Missing("organization"))
    );
    assert_eq!(
        with("v=1&org=a&port=1&kind=tcp"),
        Err(LinkError::Missing("server"))
    );
    assert_eq!(
        with("v=1&org=a&server=s&kind=tcp"),
        Err(LinkError::Missing("port"))
    );
    assert_eq!(
        with("v=1&org=a&server=s&port=1"),
        Err(LinkError::Missing("kind"))
    );
    for org in ["Acme", "-acme", "ac_me", "ac%2Fme", ""] {
        assert!(
            with(&format!("v=1&org={org}&server=s&port=1&kind=tcp")).is_err(),
            "org {org}"
        );
    }
    let long_org = "a".repeat(64);
    assert_eq!(
        with(&format!("v=1&org={long_org}&server=s&port=1&kind=tcp")),
        Err(LinkError::Invalid("organization"))
    );
    for server in ["a%2Fb", "-x", "a%20b", &"s".repeat(65)] {
        assert_eq!(
            with(&format!("v=1&org=a&server={server}&port=1&kind=tcp")),
            Err(LinkError::Invalid("server")),
            "server {server}"
        );
    }
    for port in ["0", "65536", "80a", "-1", "+80", "1.5", "99999999999"] {
        assert_eq!(
            with(&format!("v=1&org=a&server=s&port={port}&kind=tcp")),
            Err(LinkError::Invalid("port")),
            "port {port}"
        );
    }
    assert_eq!(
        with("v=1&org=a&server=s&port=65535&kind=tcp").unwrap().port,
        65535
    );
    for kind in ["udp", "HTTPS", "http", "reverse"] {
        assert_eq!(
            with(&format!("v=1&org=a&server=s&port=1&kind={kind}")),
            Err(LinkError::Invalid("kind"))
        );
    }
    for name in ["Bad", "-bad", "bad_name", "bad%20name", &"n".repeat(64)] {
        assert_eq!(
            with(&format!("v=1&org=a&server=s&port=1&kind=tcp&name={name}")),
            Err(LinkError::Invalid("name")),
            "name {name}"
        );
    }
    assert_eq!(
        with("v=1&org=a&org=b&server=s&port=1&kind=tcp"),
        Err(LinkError::Repeated("organization"))
    );
    assert_eq!(
        with("v=1&org=%zz&server=s&port=1&kind=tcp"),
        Err(LinkError::Invalid("encoding"))
    );
    assert_eq!(
        with("v=1&org=a&server=%FF&port=1&kind=tcp"),
        Err(LinkError::Invalid("encoding"))
    );
}

#[test]
fn only_open_links_go_to_the_link_handler() {
    assert!(is_open_link("reoclo://open"));
    assert!(is_open_link("reoclo://open?v=1"));
    assert!(is_open_link("reoclo://open/?v=1"));
    assert!(!is_open_link("reoclo://openers"));
    assert!(!is_open_link("reoclo://settings"));
    assert!(!is_open_link("re"));
}
