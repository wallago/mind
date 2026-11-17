//! Bearer token on `/mcp`, and OAuth access tokens when OAuth is on.

use std::{fmt, path::Path, sync::Arc};

use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use subtle::ConstantTimeEq;

use super::oauth::Issuer;
use crate::prelude::*;

/// The token every protected request must send.
#[derive(Clone)]
pub(super) struct Token(Arc<str>);

impl Token {
    /// Reads the token from `path`, without surrounding whitespace.
    ///
    /// # Errors
    ///
    /// [`Error::Config`] if the file can't be read or holds no token.
    pub(super) fn read(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .map_err(|error| Error::Config(format!("{}: {error}", path.display())))?;
        let token = raw.trim();
        if token.is_empty() {
            return Err(Error::Config(format!("{}: token is empty", path.display())));
        }
        Ok(Self(token.into()))
    }

    /// A token from a string, for tests.
    #[cfg(test)]
    pub(super) fn from_static(token: &str) -> Self {
        Self(token.into())
    }

    /// The raw token bytes, to derive keys from.
    pub(super) fn secret(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// `true` if `sent` is the token. Takes the same time wherever they differ.
    pub(super) fn matches(&self, sent: &str) -> bool {
        self.0.as_bytes().ct_eq(sent.as_bytes()).into()
    }
}

// By hand, so the token never ends up in logs.
impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(***)")
    }
}

/// The server token, and OAuth if it is on.
#[derive(Clone)]
pub(super) struct Tokens {
    /// Opens everything, sent as a header only.
    pub(super) main: Token,
    /// Issues and checks OAuth access tokens, which open what the main token
    /// opens. `None`: OAuth off.
    pub(super) oauth: Option<Issuer>,
}

impl Tokens {
    /// `true` if `sent` is the main token or a live OAuth access token.
    fn accepts(&self, sent: &str) -> bool {
        self.main.matches(sent) || self.oauth.as_ref().is_some_and(|oauth| oauth.accepts(sent))
    }
}

// By hand: the issuer holds the main token.
impl fmt::Debug for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tokens")
            .field("main", &self.main)
            .field("oauth", &self.oauth.is_some())
            .finish()
    }
}

/// The token in `Authorization: Bearer <token>`, if any.
fn bearer(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, sent)| sent.trim())
}

/// The 401 answer. With OAuth on, it points clients such as claude.ai to the
/// login (RFC 9728).
fn unauthorized(tokens: &Tokens) -> Response {
    let challenge = match &tokens.oauth {
        Some(oauth) => format!(
            "Bearer resource_metadata=\"{}\"",
            oauth.resource_metadata_url()
        ),
        None => "Bearer".to_owned(),
    };
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, challenge)],
        Json(json!({ "error": "missing or wrong bearer token" })),
    )
        .into_response()
}

/// Middleware: 401 unless the request sends `Authorization: Bearer <token>`,
/// with the main token or an OAuth access token.
pub(super) async fn require(
    State(tokens): State<Tokens>,
    request: Request,
    next: Next,
) -> Response {
    if bearer(&request).is_some_and(|sent| tokens.accepts(sent)) {
        return next.run(request).await;
    }
    unauthorized(&tokens)
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::Request,
    };
    use pretty_assertions::assert_eq;
    use tower::ServiceExt;

    use super::*;
    use crate::server::test_router;

    /// A router whose token is `secret`, without OAuth.
    fn app() -> Router {
        let tokens = Tokens {
            main: Token::from_static("secret"),
            oauth: None,
        };
        test_router("auth", Some(tokens))
    }

    /// GETs `uri` with this `Authorization` header, if any.
    async fn get(app: &Router, uri: &str, auth: Option<&str>) -> Response {
        let mut request = Request::get(uri).header(header::HOST, "localhost");
        if let Some(auth) = auth {
            request = request.header(header::AUTHORIZATION, auth);
        }
        let request = request.body(Body::empty()).expect("valid request");
        app.clone().oneshot(request).await.expect("infallible")
    }

    #[test]
    fn read_trims_and_refuses_empty() {
        let dir = std::env::temp_dir().join(format!(
            "{}-{}-token",
            env!("CARGO_PKG_NAME"),
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("token");
        std::fs::write(&path, "secret\n").expect("write");
        assert!(Token::read(&path).expect("token").matches("secret"));
        std::fs::write(&path, " \n").expect("write");
        assert!(Token::read(&path).is_err());
        assert!(Token::read(&dir.join("missing")).is_err());
    }

    #[test]
    fn debug_hides_the_token() {
        let tokens = Tokens {
            main: Token::from_static("secret"),
            oauth: None,
        };
        assert!(!format!("{tokens:?}").contains("secret"));
    }

    #[tokio::test]
    async fn mcp_needs_the_token_and_health_does_not() {
        let app = app();
        for auth in [
            None,
            Some("Bearer wrong"),
            Some("Basic secret"),
            Some("secret"),
        ] {
            let response = get(&app, "/mcp", auth).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{auth:?}");
            assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
            let bytes = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body");
            assert!(String::from_utf8_lossy(&bytes).contains("bearer token"));
        }
        // Past the token, the MCP service answers (here: a GET without a session).
        for auth in ["Bearer secret", "bearer secret"] {
            let response = get(&app, "/mcp", Some(auth)).await;
            assert_ne!(response.status(), StatusCode::UNAUTHORIZED, "{auth}");
        }
        let response = get(&app, "/health", None).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
