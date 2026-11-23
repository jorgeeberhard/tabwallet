//! Defence in depth against cross-site request forgery.
//!
//! The first line of defence is the session cookie: it is `SameSite=Lax`, so
//! the browser does not send it on cross-site POST, PUT, PATCH or DELETE
//! requests, and the front-end reaches the API through its own origin (a
//! rewrite), so there is no CORS and no other site may call the API from a
//! script. This middleware adds a second one that does not depend on the
//! cookie attribute: a state-changing request that says where it came from
//! (`Origin`, or `Referer` when there is no `Origin`) must come from the
//! front-end, whose address is `APP_URL`. Requests that say nothing (curl,
//! tests, very old browsers) pass: they are not cross-site browser requests
//! carrying our cookie, and modern browsers always send `Origin` on them.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, header},
    middleware::Next,
    response::Response,
};

use crate::{app::AppState, error::ApiError};

/// Rejects state-changing requests that come from another origin.
pub async fn require_same_origin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if is_safe(request.method()) || comes_from_front_end(&state.app_url, request.headers()) {
        Ok(next.run(request).await)
    } else {
        Err(ApiError::ForbiddenOrigin)
    }
}

/// GET, HEAD and OPTIONS never change anything.
fn is_safe(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// True when the request names the front-end as its origin, or names none.
fn comes_from_front_end(app_url: &str, headers: &HeaderMap) -> bool {
    let Some(app_origin) = origin_of(app_url) else {
        return false;
    };
    let declared = headers
        .get(header::ORIGIN)
        .or_else(|| headers.get(header::REFERER));
    let Some(declared) = declared else {
        return true;
    };
    let Some(declared) = declared.to_str().ok().and_then(origin_of) else {
        // "null" (sandboxed pages, some redirects) or something unreadable.
        return false;
    };
    declared == app_origin || (is_loopback(&app_origin) && is_loopback(&declared))
}

/// `scheme://host[:port]` of an `Origin` or of a URL, lowercased; `None` when
/// it does not look like one (for instance the literal `null`).
fn origin_of(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    let (scheme, rest) = value.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let valid_scheme = matches!(scheme, "http" | "https");
    if !valid_scheme || authority.is_empty() || authority.contains('@') {
        return None;
    }
    Some(format!("{scheme}://{authority}"))
}

/// Local development: the front-end may be opened as `localhost`,
/// `127.0.0.1`, `[::1]` or a `*.localhost` name, on any port. Only used when
/// `APP_URL` itself is one of those, never in production.
fn is_loopback(origin: &str) -> bool {
    let authority = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    let host = match authority.strip_prefix('[') {
        Some(ipv6) => ipv6.split(']').next().unwrap_or(""),
        None => authority.split(':').next().unwrap_or(""),
    };
    host == "localhost" || host.ends_with(".localhost") || host == "127.0.0.1" || host == "::1"
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(name: header::HeaderName, value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn only_the_configured_front_end_is_the_same_origin() {
        let app = "https://app.tabwallet.example/";
        assert!(comes_from_front_end(
            app,
            &headers(header::ORIGIN, "https://app.tabwallet.example")
        ));
        assert!(comes_from_front_end(
            app,
            &headers(header::ORIGIN, "HTTPS://APP.tabwallet.example")
        ));
        for foreign in [
            "https://evil.example",
            "http://app.tabwallet.example",
            "https://app.tabwallet.example:8443",
            "https://app.tabwallet.example.evil.example",
            "https://app.tabwallet.example@evil.example",
            "null",
            "",
        ] {
            assert!(
                !comes_from_front_end(app, &headers(header::ORIGIN, foreign)),
                "{foreign}"
            );
        }
    }

    #[test]
    fn the_referer_counts_only_when_there_is_no_origin() {
        let app = "https://app.tabwallet.example";
        assert!(comes_from_front_end(
            app,
            &headers(header::REFERER, "https://app.tabwallet.example/cartoes?x=1")
        ));
        assert!(!comes_from_front_end(
            app,
            &headers(
                header::REFERER,
                "https://evil.example/https://app.tabwallet.example"
            )
        ));

        let mut both = headers(header::ORIGIN, "https://evil.example");
        both.insert(
            header::REFERER,
            HeaderValue::from_static("https://app.tabwallet.example/"),
        );
        assert!(!comes_from_front_end(app, &both));
    }

    #[test]
    fn a_request_that_declares_nothing_passes() {
        assert!(comes_from_front_end(
            "https://app.tabwallet.example",
            &HeaderMap::new()
        ));
    }

    #[test]
    fn a_local_front_end_accepts_any_local_name_but_a_public_one_does_not() {
        let local = "http://localhost:3000";
        for origin in [
            "http://localhost:3000",
            "http://127.0.0.1:3000",
            "http://tw21.localhost:3100",
            "http://[::1]:3000",
        ] {
            assert!(
                comes_from_front_end(local, &headers(header::ORIGIN, origin)),
                "{origin}"
            );
        }
        assert!(!comes_from_front_end(
            local,
            &headers(header::ORIGIN, "https://evil.example")
        ));
        assert!(!comes_from_front_end(
            "https://app.tabwallet.example",
            &headers(header::ORIGIN, "http://localhost:3000")
        ));
    }

    #[test]
    fn reads_are_never_checked() {
        assert!(is_safe(&Method::GET));
        assert!(is_safe(&Method::HEAD));
        assert!(!is_safe(&Method::POST));
        assert!(!is_safe(&Method::DELETE));
    }
}
