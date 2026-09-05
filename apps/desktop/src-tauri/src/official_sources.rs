//! Public-page access only. Model credentials and manuscript data never enter this module.
use encoding_rs::{Encoding, UTF_8};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    future::Future,
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::{Host, Url};

pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_PAGES: usize = 4;
const MAX_REQUESTS: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FetchOptions {
    #[serde(default)]
    pub approved_origins: Vec<String>,
    #[serde(default)]
    pub http_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessEvent {
    pub requested_url: String,
    pub url: String,
    pub code: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingAccess {
    pub origin: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct FetchError {
    pub code: &'static str,
    pub detail: Option<String>,
}

impl FetchError {
    fn new(code: &'static str) -> Self {
        Self { code, detail: None }
    }
    fn detail(code: &'static str, detail: impl ToString) -> Self {
        Self {
            code,
            detail: Some(detail.to_string()),
        }
    }
}

pub fn source_url(value: &str) -> Result<Url, String> {
    let value = value.trim();
    if value.chars().count() > 2000 {
        return Err("OFFICIAL_INVALID_URL".into());
    }
    let mut url = Url::parse(value).map_err(|_| "OFFICIAL_INVALID_URL")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("OFFICIAL_INVALID_URL".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("OFFICIAL_CREDENTIALS".into());
    }
    if url.host().is_none() {
        return Err("OFFICIAL_INVALID_URL".into());
    }
    url.set_fragment(None);
    Ok(url)
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192
                    && (b == 168 || (b == 0 && (c == 0 || c == 2)) || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return public_ip(IpAddr::V4(mapped));
            }
            let s = ip.segments();
            // Fail closed for non-global unicast, translation/tunnel and documentation ranges.
            (s[0] & 0xe000) == 0x2000
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}

fn validate_network_url(url: &Url) -> Result<(), FetchError> {
    source_url(url.as_str()).map_err(|_| FetchError::new("OFFICIAL_INVALID_URL"))?;
    if url.port().is_some() {
        return Err(FetchError::new("OFFICIAL_PORT_BLOCKED"));
    }
    match url
        .host()
        .ok_or_else(|| FetchError::new("OFFICIAL_INVALID_URL"))?
    {
        Host::Domain(host) => {
            let host = host.trim_end_matches('.').to_ascii_lowercase();
            if !host.contains('.')
                || ["localhost", "local", "internal", "lan", "home.arpa"]
                    .iter()
                    .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
            {
                return Err(FetchError::new("OFFICIAL_PRIVATE_ADDRESS"));
            }
        }
        Host::Ipv4(ip) if !public_ip(ip.into()) => {
            return Err(FetchError::new("OFFICIAL_PRIVATE_ADDRESS"))
        }
        Host::Ipv6(ip) if !public_ip(ip.into()) => {
            return Err(FetchError::new("OFFICIAL_PRIVATE_ADDRESS"))
        }
        _ => {}
    }
    Ok(())
}

pub fn same_host(a: &Url, b: &Url) -> bool {
    a.host_str().map(|v| v.trim_end_matches('.')) == b.host_str().map(|v| v.trim_end_matches('.'))
}

fn checked_addresses(addresses: Vec<SocketAddr>) -> Result<Vec<SocketAddr>, FetchError> {
    if addresses.is_empty() {
        return Err(FetchError::new("OFFICIAL_DNS_FAILED"));
    }
    if addresses.iter().any(|address| !public_ip(address.ip())) {
        return Err(FetchError::new("OFFICIAL_PRIVATE_ADDRESS"));
    }
    Ok(addresses)
}

pub struct RawPage {
    pub status: u16,
    pub location: Option<String>,
    pub content_type: String,
    pub bytes: Vec<u8>,
}
pub trait Transport {
    fn get(&self, url: &Url) -> impl Future<Output = Result<RawPage, FetchError>> + Send;
}
pub struct PublicTransport;

fn request_error(error: reqwest::Error) -> FetchError {
    if error.is_timeout() {
        return FetchError::new("OFFICIAL_TIMEOUT");
    }
    // Do not return raw reqwest errors: they can include URL query parameters.
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    while let Some(error) = source {
        let message = error.to_string().to_ascii_lowercase();
        if message.contains("certificate") || message.contains("certvalid") {
            return FetchError::new("OFFICIAL_TLS_FAILED");
        }
        source = error.source();
    }
    FetchError::new("OFFICIAL_CONNECTION_FAILED")
}

impl Transport for PublicTransport {
    async fn get(&self, url: &Url) -> Result<RawPage, FetchError> {
        validate_network_url(url)?;
        let host = url
            .host_str()
            .ok_or_else(|| FetchError::new("OFFICIAL_INVALID_URL"))?;
        let port = url
            .port_or_known_default()
            .ok_or_else(|| FetchError::new("OFFICIAL_INVALID_URL"))?;
        let addresses = match url.host() {
            Some(Host::Ipv4(ip)) => vec![SocketAddr::new(ip.into(), port)],
            Some(Host::Ipv6(ip)) => vec![SocketAddr::new(ip.into(), port)],
            _ => tokio::time::timeout(
                Duration::from_secs(5),
                tokio::net::lookup_host((host, port)),
            )
            .await
            .map_err(|_| FetchError::new("OFFICIAL_TIMEOUT"))?
            .map_err(|_| FetchError::new("OFFICIAL_DNS_FAILED"))?
            .collect(),
        };
        let addresses = checked_addresses(addresses)?;
        // A fresh client per hop pins every connection to validated DNS results. No proxy,
        // automatic redirect, referer, cookies, credentials or reusable cross-host connection.
        let client = reqwest::Client::builder()
            .no_proxy()
            .referer(false)
            .redirect(reqwest::redirect::Policy::none())
            .resolve_to_addrs(host, &addresses)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .user_agent(concat!(
                "ManuscriptDock/",
                env!("CARGO_PKG_VERSION"),
                " official-guideline-fetch"
            ))
            .build()
            .map_err(|_| FetchError::new("OFFICIAL_CLIENT_FAILED"))?;
        let mut response = client
            .get(url.clone())
            .header("Accept-Encoding", "identity")
            .send()
            .await
            .map_err(request_error)?;
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let mut bytes = Vec::new();
        if (200..300).contains(&status) {
            if response
                .content_length()
                .is_some_and(|size| size > MAX_BYTES as u64)
            {
                return Err(FetchError::new("OFFICIAL_TOO_LARGE"));
            }
            while let Some(chunk) = response.chunk().await.map_err(request_error)? {
                if bytes.len().saturating_add(chunk.len()) > MAX_BYTES {
                    return Err(FetchError::new("OFFICIAL_TOO_LARGE"));
                }
                bytes.extend_from_slice(&chunk);
            }
        }
        Ok(RawPage {
            status,
            location,
            content_type,
            bytes,
        })
    }
}

pub struct Page {
    pub url: Url,
    pub html: String,
    pub text: String,
    pub title: String,
}

pub struct FetchSession<T: Transport> {
    seed: Url,
    options: FetchOptions,
    transport: T,
    pub events: Vec<AccessEvent>,
    pub pending: Vec<PendingAccess>,
    requests: usize,
    pub used_http: bool,
    deadline: tokio::time::Instant,
}

impl<T: Transport> FetchSession<T> {
    pub fn new(seed: Url, options: FetchOptions, transport: T) -> Result<Self, String> {
        if options.approved_origins.len() > 8 || options.http_origins.len() > 8 {
            return Err("OFFICIAL_INVALID_URL".into());
        }
        for value in options.approved_origins.iter().chain(&options.http_origins) {
            let url = source_url(value)?;
            validate_network_url(&url).map_err(|e| e.code.to_owned())?;
            if url.origin().ascii_serialization() != *value {
                return Err("OFFICIAL_INVALID_URL".into());
            }
        }
        Ok(Self {
            seed,
            options,
            transport,
            events: vec![],
            pending: vec![],
            requests: 0,
            used_http: false,
            deadline: tokio::time::Instant::now() + Duration::from_secs(90),
        })
    }

    fn event(&mut self, requested: &Url, url: &Url, code: &str, detail: Option<String>) {
        self.events.push(AccessEvent {
            requested_url: requested.to_string(),
            url: url.to_string(),
            code: code.into(),
            detail,
        });
    }
    fn pending(&mut self, url: &Url, kind: &str) {
        let pending = PendingAccess {
            origin: url.origin().ascii_serialization(),
            kind: kind.into(),
        };
        if !self.pending.contains(&pending) {
            self.pending.push(pending);
        }
    }

    async fn chain(
        &mut self,
        requested: &Url,
        mut current: Url,
    ) -> Result<(Url, RawPage), FetchError> {
        let mut seen = BTreeSet::new();
        for _ in 0..=3 {
            if let Err(error) = validate_network_url(&current) {
                self.event(requested, &current, error.code, error.detail.clone());
                return Err(error);
            }
            let origin = current.origin().ascii_serialization();
            if !same_host(&self.seed, &current) && !self.options.approved_origins.contains(&origin)
            {
                self.pending(&current, "origin");
                self.event(requested, &current, "OFFICIAL_ORIGIN_CONFIRMATION", None);
                return Err(FetchError::new("OFFICIAL_ORIGIN_CONFIRMATION"));
            }
            if current.scheme() == "http" && !self.options.http_origins.contains(&origin) {
                self.pending(&current, "http");
                self.event(requested, &current, "OFFICIAL_HTTP_CONFIRMATION", None);
                return Err(FetchError::new("OFFICIAL_HTTP_CONFIRMATION"));
            }
            if !seen.insert(current.to_string()) {
                return Err(FetchError::new("OFFICIAL_REDIRECT_LIMIT"));
            }
            if self.requests >= MAX_REQUESTS {
                return Err(FetchError::new("OFFICIAL_REQUEST_LIMIT"));
            }
            self.requests += 1;
            self.used_http |= current.scheme() == "http";
            self.event(requested, &current, "OFFICIAL_REQUESTED", None);
            let response =
                match tokio::time::timeout_at(self.deadline, self.transport.get(&current))
                    .await
                    .unwrap_or_else(|_| Err(FetchError::new("OFFICIAL_TIMEOUT")))
                {
                    Ok(response) => response,
                    Err(error) => {
                        self.event(requested, &current, error.code, error.detail.clone());
                        return Err(error);
                    }
                };
            if [301, 302, 303, 307, 308].contains(&response.status) {
                self.event(
                    requested,
                    &current,
                    "OFFICIAL_REDIRECT",
                    Some(response.status.to_string()),
                );
                current = current
                    .join(
                        response
                            .location
                            .as_deref()
                            .ok_or_else(|| FetchError::new("OFFICIAL_BAD_REDIRECT"))?,
                    )
                    .map_err(|_| FetchError::new("OFFICIAL_BAD_REDIRECT"))?;
                current.set_fragment(None);
                continue;
            }
            if !(200..300).contains(&response.status) {
                let error = FetchError::detail("OFFICIAL_HTTP_STATUS", response.status);
                self.event(requested, &current, error.code, error.detail.clone());
                return Err(error);
            }
            self.event(
                requested,
                &current,
                "OFFICIAL_RECEIVED",
                Some(response.status.to_string()),
            );
            return Ok((current, response));
        }
        Err(FetchError::new("OFFICIAL_REDIRECT_LIMIT"))
    }

    async fn raw(&mut self, requested: &Url) -> Result<(Url, RawPage), FetchError> {
        validate_network_url(requested)?;
        if requested.scheme() != "http" {
            return self.chain(requested, requested.clone()).await;
        }
        let mut secure = requested.clone();
        secure
            .set_scheme("https")
            .map_err(|_| FetchError::new("OFFICIAL_INVALID_URL"))?;
        match self.chain(requested, secure).await {
            Ok(page) => Ok(page),
            Err(error)
                if matches!(
                    error.code,
                    "OFFICIAL_CONNECTION_FAILED"
                        | "OFFICIAL_TLS_FAILED"
                        | "OFFICIAL_TIMEOUT"
                        | "OFFICIAL_HTTP_STATUS"
                        | "OFFICIAL_DNS_FAILED"
                ) =>
            {
                self.chain(requested, requested.clone()).await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn page(&mut self, requested: Url) -> Result<Page, FetchError> {
        let result = self.page_inner(&requested).await;
        if let Err(error) = &result {
            self.event(&requested, &requested, error.code, error.detail.clone());
        }
        result
    }

    async fn page_inner(&mut self, requested: &Url) -> Result<Page, FetchError> {
        let (url, raw) = self.raw(requested).await?;
        let mime = raw
            .content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !matches!(
            mime.as_str(),
            "text/html" | "application/xhtml+xml" | "text/plain"
        ) {
            return Err(FetchError::new("OFFICIAL_UNSUPPORTED_FORMAT"));
        }
        let html = decode_page(&raw.bytes, &raw.content_type)?;
        let text = if mime == "text/plain" {
            html.clone()
        } else {
            super::html_to_plain_text(&html)
        };
        if text.chars().count() < 20 {
            return Err(FetchError::new("OFFICIAL_NO_TEXT"));
        }
        let document = Html::parse_document(&html);
        let title = document
            .select(&Selector::parse("title").expect("static selector"))
            .next()
            .map(|node| node.text().collect::<String>())
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| "Official journal page".into());
        self.event(requested, &url, "OFFICIAL_CAPTURED", None);
        Ok(Page {
            url,
            html,
            text,
            title,
        })
    }

    pub async fn hydrate(&mut self, page: &mut Page) -> bool {
        let Some(news_id) = super::html_input_value(&page.html, "newsId") else {
            return true;
        };
        if news_id.is_empty()
            || news_id.len() > 100
            || !news_id.chars().all(|c| c.is_ascii_alphanumeric())
        {
            self.event(&page.url, &page.url, "OFFICIAL_DYNAMIC_UNAVAILABLE", None);
            return false;
        }
        let base_path =
            super::html_input_value(&page.html, "basePath").unwrap_or_else(|| "/".into());
        let Ok(mut url) = page.url.join(&format!(
            "{}/data/news/newsData",
            base_path.trim_end_matches('/')
        )) else {
            self.event(&page.url, &page.url, "OFFICIAL_DYNAMIC_UNAVAILABLE", None);
            return false;
        };
        // Dynamic endpoints are GET-only and must remain on the page's exact host.
        if !same_host(&url, &page.url) {
            self.event(&page.url, &url, "OFFICIAL_DYNAMIC_UNAVAILABLE", None);
            return false;
        }
        url.query_pairs_mut().append_pair("id", &news_id);
        match self.raw(&url).await {
            Ok((final_url, raw)) => {
                if let Some((title, text)) = super::dynamic_news_content(&raw.bytes) {
                    page.text = text;
                    if let Some(title) = title {
                        page.title = title;
                    }
                    // The hash and text came from this endpoint, not from the HTML shell.
                    page.url = final_url.clone();
                    self.event(&url, &final_url, "OFFICIAL_CAPTURED", None);
                    return true;
                } else {
                    self.event(&url, &final_url, "OFFICIAL_DYNAMIC_UNAVAILABLE", None);
                }
            }
            Err(error) => self.event(&url, &url, error.code, error.detail),
        }
        false
    }
}

fn charset(value: &str) -> Option<&'static Encoding> {
    value.split(';').find_map(|part| {
        let (key, label) = part.trim().split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("charset")
            .then(|| Encoding::for_label(label.trim().trim_matches(['\'', '"']).as_bytes()))
            .flatten()
    })
}

fn decode_page(bytes: &[u8], content_type: &str) -> Result<String, FetchError> {
    if bytes.len() > MAX_BYTES {
        return Err(FetchError::new("OFFICIAL_TOO_LARGE"));
    }
    let bom = Encoding::for_bom(bytes).map(|(encoding, _)| encoding);
    let encoding = bom
        .or_else(|| charset(content_type))
        .or_else(|| {
            let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
            let document = Html::parse_document(&prefix);
            document
                .select(&Selector::parse("meta").expect("static selector"))
                .find_map(|meta| {
                    meta.value()
                        .attr("charset")
                        .and_then(|v| Encoding::for_label(v.as_bytes()))
                        .or_else(|| meta.value().attr("content").and_then(charset))
                })
        })
        .unwrap_or(UTF_8);
    let (decoded, _, malformed) = encoding.decode(bytes);
    if malformed {
        return Err(FetchError::new("OFFICIAL_ENCODING_FAILED"));
    }
    Ok(decoded.into_owned())
}

pub fn instruction_links(base: &Url, html: &str) -> Vec<Url> {
    let document = Html::parse_document(html);
    let link_base = document
        .select(&Selector::parse("base[href]").expect("static selector"))
        .next()
        .and_then(|node| base.join(node.value().attr("href")?).ok())
        .unwrap_or_else(|| base.clone());
    let mut links = Vec::new();
    for node in document.select(&Selector::parse("a[href]").expect("static selector")) {
        let href = node.value().attr("href").unwrap_or("");
        if !super::instruction_page_hint(href)
            && !super::instruction_page_hint(&node.text().collect::<String>())
        {
            continue;
        }
        let Ok(url) = link_base.join(href) else {
            continue;
        };
        let Ok(url) = source_url(url.as_str()) else {
            continue;
        };
        if !links.contains(&url) && &url != base {
            links.push(url);
        }
        if links.len() == MAX_PAGES - 1 {
            break;
        }
    }
    links
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, sync::Mutex};

    struct FixtureTransport {
        replies: Mutex<VecDeque<Result<RawPage, FetchError>>>,
        urls: Mutex<Vec<String>>,
    }
    impl FixtureTransport {
        fn new(replies: Vec<Result<RawPage, FetchError>>) -> Self {
            Self {
                replies: Mutex::new(replies.into()),
                urls: Mutex::new(vec![]),
            }
        }
    }
    impl Transport for FixtureTransport {
        async fn get(&self, url: &Url) -> Result<RawPage, FetchError> {
            self.urls.lock().unwrap().push(url.to_string());
            self.replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected network request")
        }
    }
    fn html() -> Result<RawPage, FetchError> {
        Ok(RawPage {
            status: 200,
            location: None,
            content_type: "text/html; charset=utf-8".into(),
            bytes: b"<title>Author guide</title><p>A separate title page is required.</p>".to_vec(),
        })
    }
    fn redirect(url: &str) -> Result<RawPage, FetchError> {
        Ok(RawPage {
            status: 302,
            location: Some(url.into()),
            content_type: String::new(),
            bytes: vec![],
        })
    }
    fn session(
        url: &str,
        options: FetchOptions,
        replies: Vec<Result<RawPage, FetchError>>,
    ) -> FetchSession<FixtureTransport> {
        FetchSession::new(
            source_url(url).unwrap(),
            options,
            FixtureTransport::new(replies),
        )
        .unwrap()
    }

    #[test]
    fn storage_accepts_http_without_granting_network_permission() {
        for input in [
            " HTTP://journal.example/guide ",
            "https://journal.example/guide",
            "http://[::1]/local-policy",
        ] {
            assert!(source_url(input).is_ok());
        }
        for input in [
            "https://",
            "https://user:password@journal.example",
            "file:///tmp/file",
            "javascript:alert(1)",
        ] {
            assert!(source_url(input).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_private_reserved_ipv4_ipv6_and_mixed_dns_answers() {
        for input in [
            "https://[::1]/",
            "https://[fd00::1]/",
            "https://[fe80::1]/",
            "https://[::ffff:127.0.0.1]/",
            "https://[2002:7f00:1::]/",
            "https://127.1/",
            "https://2130706433/",
            "https://100.64.1.1/",
            "https://192.0.2.1/",
            "https://journal.local/",
            "https://localhost./",
            "https://journal.example:8443/",
        ] {
            assert!(
                validate_network_url(&source_url(input).unwrap()).is_err(),
                "{input}"
            );
        }
        assert!(checked_addresses(vec![
            "8.8.8.8:443".parse().unwrap(),
            "127.0.0.1:443".parse().unwrap()
        ])
        .is_err());
        assert!(checked_addresses(vec!["[fd00::1]:443".parse().unwrap()]).is_err());
        assert!(checked_addresses(vec![]).is_err());
        assert!(checked_addresses(vec![
            "8.8.8.8:443".parse().unwrap(),
            "[2606:4700:4700::1111]:443".parse().unwrap()
        ])
        .is_ok());
    }

    #[tokio::test]
    async fn upgrades_http_and_keeps_original_and_final_urls() {
        let url = source_url("http://journal.example/guide").unwrap();
        let mut s = session(url.as_str(), FetchOptions::default(), vec![html()]);
        let page = s.page(url.clone()).await.unwrap();
        assert_eq!(page.url.scheme(), "https");
        assert!(!s.used_http);
        assert_eq!(s.events.last().unwrap().requested_url, url.as_str());
        assert_eq!(
            s.events.last().unwrap().url,
            "https://journal.example/guide"
        );
    }

    #[tokio::test]
    async fn https_failure_never_silently_downgrades_or_reuses_consent() {
        let input = "http://journal.example/guide";
        let mut s = session(
            input,
            FetchOptions::default(),
            vec![Err(FetchError::new("OFFICIAL_TLS_FAILED"))],
        );
        assert_eq!(
            s.page(source_url(input).unwrap()).await.err().unwrap().code,
            "OFFICIAL_HTTP_CONFIRMATION"
        );
        assert_eq!(s.transport.urls.lock().unwrap().len(), 1);
        assert_eq!(s.pending[0].origin, "http://journal.example");
        let mut allowed = session(
            input,
            FetchOptions {
                http_origins: vec!["http://journal.example".into()],
                ..Default::default()
            },
            vec![Err(FetchError::new("OFFICIAL_TLS_FAILED")), html()],
        );
        assert_eq!(
            allowed
                .page(source_url(input).unwrap())
                .await
                .unwrap()
                .url
                .scheme(),
            "http"
        );
        assert!(allowed.used_http);
        // A new invocation has no access to the previous invocation's grants.
        let fresh = session(input, FetchOptions::default(), vec![]);
        assert!(fresh.options.http_origins.is_empty());
    }

    #[tokio::test]
    async fn redirects_require_exact_origin_confirmation_and_reject_private_targets() {
        let seed = "https://journal.example/guide";
        let mut s = session(
            seed,
            FetchOptions::default(),
            vec![redirect("https://authors.journal.example/guide")],
        );
        assert_eq!(
            s.page(source_url(seed).unwrap()).await.err().unwrap().code,
            "OFFICIAL_ORIGIN_CONFIRMATION"
        );
        assert_eq!(s.transport.urls.lock().unwrap().len(), 1);
        let mut allowed = session(
            seed,
            FetchOptions {
                approved_origins: vec!["https://authors.journal.example".into()],
                ..Default::default()
            },
            vec![redirect("https://authors.journal.example/guide"), html()],
        );
        assert!(allowed.page(source_url(seed).unwrap()).await.is_ok());
        let mut private = session(
            seed,
            FetchOptions::default(),
            vec![redirect("https://[::1]/guide")],
        );
        assert_eq!(
            private
                .page(source_url(seed).unwrap())
                .await
                .err()
                .unwrap()
                .code,
            "OFFICIAL_PRIVATE_ADDRESS"
        );
        assert!(private.pending.is_empty());
    }

    #[tokio::test]
    async fn downgrades_loops_and_budgets_are_bounded() {
        let seed = "https://journal.example/guide";
        let mut downgrade = session(
            seed,
            FetchOptions::default(),
            vec![redirect("http://journal.example/guide")],
        );
        assert_eq!(
            downgrade
                .page(source_url(seed).unwrap())
                .await
                .err()
                .unwrap()
                .code,
            "OFFICIAL_HTTP_CONFIRMATION"
        );
        let mut looping = session(seed, FetchOptions::default(), vec![redirect(seed)]);
        assert_eq!(
            looping
                .page(source_url(seed).unwrap())
                .await
                .err()
                .unwrap()
                .code,
            "OFFICIAL_REDIRECT_LIMIT"
        );
        let mut budget = session(seed, FetchOptions::default(), vec![]);
        budget.requests = MAX_REQUESTS;
        assert_eq!(
            budget
                .page(source_url(seed).unwrap())
                .await
                .err()
                .unwrap()
                .code,
            "OFFICIAL_REQUEST_LIMIT"
        );
    }

    #[test]
    fn decodes_legacy_chinese_and_respects_bom_header_and_meta() {
        let text = "投稿指南：所有作者必须提供标题页。";
        for encoding in [encoding_rs::GBK, encoding_rs::GB18030] {
            let (bytes, _, _) = encoding.encode(text);
            assert_eq!(
                decode_page(&bytes, &format!("text/html; charset={}", encoding.name())).unwrap(),
                text
            );
            let page = format!("<meta charset='{}'><p>{text}</p>", encoding.name());
            let (bytes, _, _) = encoding.encode(&page);
            assert!(decode_page(&bytes, "text/html").unwrap().contains(text));
        }
        let mut bom = vec![0xef, 0xbb, 0xbf];
        bom.extend_from_slice(text.as_bytes());
        assert_eq!(decode_page(&bom, "text/html; charset=gbk").unwrap(), text);
        assert!(decode_page(&[0xff, 0xfe, 0x01], "text/plain").is_err());
    }

    #[test]
    fn parses_entities_base_urls_http_and_external_guide_candidates() {
        let base = source_url("https://journal.example/").unwrap();
        let links = instruction_links(&base, "<base href='/docs/'><a href='guide?id=1&amp;lang=zh'>作者指南</a><a href='http://journal.example/guide'>投稿指南</a><a href='https://authors.publisher.example/guide'>Author guidelines</a>");
        assert_eq!(links.len(), 3);
        assert_eq!(
            links[0].as_str(),
            "https://journal.example/docs/guide?id=1&lang=zh"
        );
        assert_eq!(links[1].scheme(), "http");
    }

    #[tokio::test]
    async fn reports_status_pdf_and_dynamic_endpoint_failure_without_hiding_them() {
        let seed = "https://journal.example/guide";
        let mut denied = session(
            seed,
            FetchOptions::default(),
            vec![Ok(RawPage {
                status: 403,
                location: None,
                content_type: String::new(),
                bytes: vec![],
            })],
        );
        assert_eq!(
            denied
                .page(source_url(seed).unwrap())
                .await
                .err()
                .unwrap()
                .detail
                .as_deref(),
            Some("403")
        );
        let mut pdf = session(
            seed,
            FetchOptions::default(),
            vec![Ok(RawPage {
                status: 200,
                location: None,
                content_type: "application/pdf".into(),
                bytes: vec![],
            })],
        );
        assert_eq!(
            pdf.page(source_url(seed).unwrap())
                .await
                .err()
                .unwrap()
                .code,
            "OFFICIAL_UNSUPPORTED_FORMAT"
        );
        let mut s = session(
            seed,
            FetchOptions::default(),
            vec![Err(FetchError::new("OFFICIAL_CONNECTION_FAILED"))],
        );
        let mut page = Page {
            url: source_url(seed).unwrap(),
            html: "<input id='newsId' value='42'>".into(),
            title: "guide".into(),
            text: "shell".into(),
        };
        assert!(!s.hydrate(&mut page).await);
        assert!(s
            .events
            .iter()
            .any(|event| event.code == "OFFICIAL_CONNECTION_FAILED"));
    }

    #[tokio::test]
    #[ignore = "Live public-site smoke check; run explicitly, separate from deterministic tests"]
    async fn live_public_journal_https_probe() {
        for input in ["http://jcip.cipsc.org.cn/", "https://crad.ict.ac.cn/"] {
            let seed = source_url(input).unwrap();
            let mut session =
                FetchSession::new(seed.clone(), FetchOptions::default(), PublicTransport).unwrap();
            let page = session.page(seed).await;
            println!(
                "{}",
                serde_json::json!({"requestedUrl": input, "captured": page.is_ok(), "events": session.events, "pending": session.pending})
            );
        }
    }
}
