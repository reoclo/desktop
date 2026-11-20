use reoclo_desktop::ui::{State, view};
use rune::{Env, Runtime, TextEdit};

#[test]
fn unsafe_domains_never_enable_trust_or_queue_an_api_request() {
    for domain in [
        "company.com",
        "host.local",
        "lan",
        "*.test",
        "bad..test",
        "test.evil.com",
    ] {
        let mut state = State::fixture("certificates", "loaded").unwrap();
        state.trust_domain = TextEdit::new(domain);
        let mut rt = Runtime::new(state, view, (1100.0, 900.0), Env::new(false, 1.0));
        rt.frame(0.0);
        let (x, y) = rt.find("trust-domain").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.02);
        assert!(rt.state.outbox.is_empty(), "{domain}");
        assert!(rt.find("trust-domain-validation").is_some());
    }
}
