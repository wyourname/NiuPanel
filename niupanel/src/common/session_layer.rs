use axum::http::{Request, Response, StatusCode, header::HOST};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use time::{Duration, OffsetDateTime};
use tower::{Layer, Service};
use tower_cookies::{Cookie, Cookies, Key, cookie::SameSite};
use tower_sessions::{
    Session, SessionStore,
    session::{Expiry, Id},
};

#[derive(Clone)]
pub struct DynamicSessionLayer<Store: SessionStore> {
    session_store: Arc<Store>,
    key: Key,
    fallback_cookie_name: Arc<str>,
    http_only: bool,
    same_site: SameSite,
    expiry: Option<Expiry>,
    secure: bool,
    path: Arc<str>,
    domain: Option<Arc<str>>,
    always_save: bool,
}

impl<Store: SessionStore> DynamicSessionLayer<Store> {
    pub fn new(session_store: Store, key: Key, fallback_cookie_name: impl Into<Arc<str>>) -> Self {
        Self {
            session_store: Arc::new(session_store),
            key,
            fallback_cookie_name: fallback_cookie_name.into(),
            http_only: true,
            same_site: SameSite::Lax,
            expiry: None,
            secure: true,
            path: Arc::from("/"),
            domain: None,
            always_save: false,
        }
    }

    pub fn with_expiry(mut self, expiry: Expiry) -> Self {
        self.expiry = Some(expiry);
        self
    }

    pub fn with_http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    pub fn with_same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }

    pub fn with_secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
}

impl<S, Store: SessionStore> Layer<S> for DynamicSessionLayer<Store> {
    type Service = DynamicSessionManager<S, Store>;

    fn layer(&self, inner: S) -> Self::Service {
        DynamicSessionManager {
            inner,
            session_store: self.session_store.clone(),
            key: self.key.clone(),
            fallback_cookie_name: self.fallback_cookie_name.clone(),
            http_only: self.http_only,
            same_site: self.same_site,
            expiry: self.expiry,
            secure: self.secure,
            path: self.path.clone(),
            domain: self.domain.clone(),
            always_save: self.always_save,
        }
    }
}

#[derive(Clone)]
pub struct DynamicSessionManager<S, Store: SessionStore> {
    inner: S,
    session_store: Arc<Store>,
    key: Key,
    fallback_cookie_name: Arc<str>,
    http_only: bool,
    same_site: SameSite,
    expiry: Option<Expiry>,
    secure: bool,
    path: Arc<str>,
    domain: Option<Arc<str>>,
    always_save: bool,
}

impl<ReqBody, ResBody, S, Store> Service<Request<ReqBody>> for DynamicSessionManager<S, Store>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Default + Send,
    Store: SessionStore,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let session_store = self.session_store.clone();
        let key = self.key.clone();
        let fallback_cookie_name = self.fallback_cookie_name.clone();
        let http_only = self.http_only;
        let same_site = self.same_site;
        let expiry = self.expiry;
        let secure = self.secure;
        let path = self.path.clone();
        let domain = self.domain.clone();
        let always_save = self.always_save;

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let Some(cookies) = req.extensions().get::<Cookies>().cloned() else {
                let mut res = Response::default();
                *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                return Ok(res);
            };

            let host = req
                .headers()
                .get(HOST)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            let cookie_name = build_cookie_name(host, &fallback_cookie_name);

            let session_cookie = cookies
                .signed(&key)
                .get(&cookie_name)
                .map(Cookie::into_owned);
            let session_id = session_cookie
                .as_ref()
                .and_then(|cookie| cookie.value().parse::<Id>().ok());
            let session = Session::new(session_id, session_store, expiry);

            req.extensions_mut().insert(session.clone());

            let res = inner.call(req).await?;

            let modified = session.is_modified();
            let empty = session.is_empty().await;

            match session_cookie {
                Some(mut cookie) if empty => {
                    cookie.set_path(path.to_string());
                    if let Some(domain) = domain.clone() {
                        cookie.set_domain(domain.to_string());
                    }
                    cookies.signed(&key).remove(cookie);
                }
                _ if (modified || always_save) && !empty && !res.status().is_server_error() => {
                    if session.save().await.is_err() {
                        let mut res = Response::default();
                        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return Ok(res);
                    }

                    let Some(session_id) = session.id() else {
                        let mut res = Response::default();
                        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        return Ok(res);
                    };

                    let cookie = build_session_cookie(
                        cookie_name,
                        session_id,
                        session.expiry(),
                        http_only,
                        same_site,
                        secure,
                        &path,
                        domain.as_deref(),
                    );
                    cookies.signed(&key).add(cookie);
                }
                _ => {}
            }

            Ok(res)
        })
    }
}

fn build_cookie_name(host: &str, fallback: &str) -> String {
    let normalized = host.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return fallback.to_string();
    }

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("niupanel.{}.sid", &digest[..12])
}

fn build_session_cookie(
    name: String,
    session_id: Id,
    expiry: Option<Expiry>,
    http_only: bool,
    same_site: SameSite,
    secure: bool,
    path: &str,
    domain: Option<&str>,
) -> Cookie<'static> {
    let mut cookie_builder = Cookie::build((name, session_id.to_string()))
        .http_only(http_only)
        .same_site(same_site)
        .secure(secure)
        .path(Cow::Owned(path.to_string()));

    cookie_builder = match expiry {
        Some(Expiry::OnInactivity(duration)) => cookie_builder.max_age(duration),
        Some(Expiry::AtDateTime(datetime)) => {
            cookie_builder.max_age(datetime - OffsetDateTime::now_utc())
        }
        Some(Expiry::OnSessionEnd) | None => cookie_builder,
    };

    if let Some(domain) = domain {
        cookie_builder = cookie_builder.domain(Cow::Owned(domain.to_string()));
    }

    cookie_builder.build()
}

pub fn default_cookie_name_from_session_key(session_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_key.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("niupanel.{}.sid", &digest[..12])
}

pub fn default_session_expiry() -> Expiry {
    Expiry::OnInactivity(Duration::days(30))
}
