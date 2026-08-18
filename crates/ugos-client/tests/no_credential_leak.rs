//! Regression tests: credentials must not reach any rendered output.
//!
//! Error text and debug output get pasted into bug reports. A session token
//! stays valid for 25 minutes, so publishing one hands over a live session.

// The no-panic rule exists for library code, where a caller cannot recover
// from it. A failing setup step in a test should abort that test.
#![allow(clippy::expect_used)]

use ugos_client::{Credentials, Session, TlsPolicy, UgosClient};

const TOKEN: &str = "TOKEN_c0ffee_SECRET";
const PASSWORD: &str = "PASSWORD_hunter2_SECRET";

fn client() -> UgosClient {
    let creds = Credentials {
        username: "someone".into(),
        password: PASSWORD.into(),
    };
    let session = Session {
        token: TOKEN.into(),
        public_key: String::new(),
    };
    UgosClient::from_session("nas.example", 9443, creds, session, &TlsPolicy::Insecure)
        .expect("client should build")
}

#[test]
fn debug_output_hides_the_token_and_password() {
    let shown = format!("{:?}", client());
    assert!(!shown.contains(TOKEN), "token leaked: {shown}");
    assert!(!shown.contains(PASSWORD), "password leaked: {shown}");
}

#[test]
fn a_failed_request_hides_the_token() {
    // Points at a port nothing listens on, so the request fails at connect
    // time with the token already appended to the URL.
    let creds = Credentials {
        username: "someone".into(),
        password: PASSWORD.into(),
    };
    let session = Session {
        token: TOKEN.into(),
        public_key: String::new(),
    };
    let unreachable =
        UgosClient::from_session("127.0.0.1", 1, creds, session, &TlsPolicy::Insecure)
            .expect("client should build");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let result: Result<serde_json::Value, _> =
        runtime.block_on(async { unreachable.get("sysinfo/machine/common").await });

    let err = result.expect_err("request to a dead port should fail");
    let rendered = format!("{err}");
    assert!(
        !rendered.contains(TOKEN),
        "token leaked in Display: {rendered}"
    );

    // The source chain is printed by error reporters too.
    let mut source: Option<&dyn std::error::Error> = std::error::Error::source(&err);
    while let Some(inner) = source {
        let text = format!("{inner}");
        assert!(!text.contains(TOKEN), "token leaked in source: {text}");
        source = std::error::Error::source(inner);
    }

    // The path is what makes the message useful; it must survive.
    assert!(
        rendered.contains("sysinfo/machine/common"),
        "path context lost: {rendered}"
    );
}
