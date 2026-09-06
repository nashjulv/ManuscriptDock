//! Public-page access only. Model credentials and manuscript data never enter this module.
use encoding_rs::{Encoding, UTF_8};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    future::Future,
    io::Read,
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
    // Read legacy access records; this field no longer grants or gates HTTP access.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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

fn virtual_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.octets()[0] == 198 && matches!(ip.octets()[1], 18 | 19),
        IpAddr::V6(ip) => ip.to_ipv4_mapped().is_some_and(|ip| virtual_ip(ip.into())),
    }
}

fn needs_encrypted_dns(addresses: &[SocketAddr]) -> Result<bool, FetchError> {
    // A mixed private/virtual answer must not use the resolver as an escape hatch.
    if addresses
        .iter()
        .any(|a| !public_ip(a.ip()) && !virtual_ip(a.ip()))
    {
        return Err(FetchError::new("OFFICIAL_PRIVATE_ADDRESS"));
    }
    Ok(addresses.iter().any(|a| virtual_ip(a.ip())))
}

#[derive(Deserialize)]
struct DnsAnswer {
    #[serde(rename = "type")]
    kind: u16,
    data: String,
}

#[derive(Deserialize)]
struct DnsResponse {
    #[serde(rename = "Status")]
    status: u16,
    #[serde(rename = "TC", default)]
    truncated: bool,
    #[serde(rename = "Answer", default)]
    answers: Vec<DnsAnswer>,
}

fn dns_addresses(bytes: &[u8], port: u16) -> Result<Vec<SocketAddr>, FetchError> {
    let response: DnsResponse = serde_json::from_slice(bytes)
        .map_err(|_| FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"))?;
    if response.status != 0 || response.truncated {
        return Err(FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"));
    }
    let mut addresses = Vec::new();
    for answer in response.answers {
        if matches!(answer.kind, 1 | 28) {
            let ip: IpAddr = answer
                .data
                .parse()
                .map_err(|_| FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"))?;
            if (answer.kind == 1) != ip.is_ipv4() {
                return Err(FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"));
            }
            addresses.push(SocketAddr::new(ip, port));
        }
    }
    // Empty AAAA answers are normal; validate the combined A + AAAA result later.
    if !addresses.is_empty() {
        checked_addresses(addresses.clone())?;
    }
    Ok(addresses)
}

async fn encrypted_addresses(host: &str, port: u16) -> Result<Vec<SocketAddr>, FetchError> {
    // Bootstrap independently of system DNS. TLS still authenticates the resolver hostname.
    let bootstrap = [
        "1.1.1.1:443".parse().unwrap(),
        "1.0.0.1:443".parse().unwrap(),
    ];
    let client = reqwest::Client::builder()
        .no_proxy()
        .referer(false)
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs("cloudflare-dns.com", &bootstrap)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"))?;
    let query = |kind: &'static str| {
        let client = &client;
        async move {
            let mut response = client
                .get("https://cloudflare-dns.com/dns-query")
                .query(&[("name", host), ("type", kind)])
                .header("Accept", "application/dns-json")
                .send()
                .await
                .map_err(|_| FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"))?;
            if !response.status().is_success() {
                return Err(FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"))?
            {
                if bytes.len().saturating_add(chunk.len()) > 32 * 1024 {
                    return Err(FetchError::new("OFFICIAL_ENCRYPTED_DNS_FAILED"));
                }
                bytes.extend_from_slice(&chunk);
            }
            dns_addresses(&bytes, port)
        }
    };
    let (a, aaaa) = tokio::join!(query("A"), query("AAAA"));
    let mut addresses = a?;
    addresses.extend(aaaa?);
    checked_addresses(addresses)
}

pub struct RawPage {
    pub status: u16,
    pub location: Option<String>,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

fn decompress_page(bytes: Vec<u8>) -> Result<(Vec<u8>, bool), FetchError> {
    // Some public sites send gzip bytes even for identity, without Content-Encoding.
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Ok((bytes, false));
    }
    let mut decoded = Vec::new();
    flate2::read::MultiGzDecoder::new(bytes.as_slice())
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut decoded)
        .map_err(|_| FetchError::new("OFFICIAL_COMPRESSION_FAILED"))?;
    if decoded.len() > MAX_BYTES {
        return Err(FetchError::new("OFFICIAL_TOO_LARGE"));
    }
    Ok((decoded, true))
}
pub trait Transport {
    fn get(
        &self,
        url: &Url,
        events: &mut Vec<AccessEvent>,
    ) -> impl Future<Output = Result<RawPage, FetchError>> + Send;
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
    async fn get(&self, url: &Url, events: &mut Vec<AccessEvent>) -> Result<RawPage, FetchError> {
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
        let addresses = if needs_encrypted_dns(&addresses)? {
            events.push(AccessEvent {
                requested_url: url.to_string(),
                url: url.to_string(),
                code: "OFFICIAL_VIRTUAL_DNS".into(),
                detail: None,
            });
            let addresses = encrypted_addresses(host, port).await?;
            events.push(AccessEvent {
                requested_url: url.to_string(),
                url: url.to_string(),
                code: "OFFICIAL_DNS_RECOVERED".into(),
                detail: None,
            });
            addresses
        } else {
            checked_addresses(addresses)?
        };
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
        let (bytes, compressed) = decompress_page(bytes)?;
        if compressed {
            events.push(AccessEvent {
                requested_url: url.to_string(),
                url: url.to_string(),
                code: "OFFICIAL_GZIP_DECODED".into(),
                detail: None,
            });
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
            if !same_host(&self.seed, &current)
                && !self.options.approved_origins.iter().any(|origin| {
                    source_url(origin).is_ok_and(|approved| same_host(&approved, &current))
                })
            {
                self.pending(&current, "origin");
                self.event(requested, &current, "OFFICIAL_ORIGIN_CONFIRMATION", None);
                return Err(FetchError::new("OFFICIAL_ORIGIN_CONFIRMATION"));
            }
            if !seen.insert(current.to_string()) {
                return Err(FetchError::new("OFFICIAL_REDIRECT_LIMIT"));
            }
            if self.requests >= MAX_REQUESTS {
                return Err(FetchError::new("OFFICIAL_REQUEST_LIMIT"));
            }
            self.requests += 1;
            self.used_http |= current.scheme() == "http";
            if current.scheme() == "http" {
                self.event(requested, &current, "OFFICIAL_HTTP_USED", None);
            }
            self.event(requested, &current, "OFFICIAL_REQUESTED", None);
            let mut transport_events = Vec::new();
            let response = tokio::time::timeout_at(
                self.deadline,
                self.transport.get(&current, &mut transport_events),
            )
            .await
            .unwrap_or_else(|_| Err(FetchError::new("OFFICIAL_TIMEOUT")));
            for event in transport_events {
                self.event(requested, &current, &event.code, event.detail);
            }
            let response = match response {
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
        match self.chain(requested, requested.clone()).await {
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
                let mut alternate = requested.clone();
                alternate
                    .set_scheme(if requested.scheme() == "http" {
                        "https"
                    } else {
                        "http"
                    })
                    .map_err(|_| FetchError::new("OFFICIAL_INVALID_URL"))?;
                self.chain(requested, alternate).await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn page(&mut self, requested: Url) -> Result<Page, FetchError> {
        let result = self.page_inner(&requested).await;
        if let Err(error) = &result {
            if !self.events.last().is_some_and(|event| {
                event.requested_url == requested.as_str()
                    && event.code == error.code
                    && event.detail == error.detail
            }) {
                self.event(&requested, &requested, error.code, error.detail.clone());
            }
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
        let (html, inferred_encoding) = decode_page_with_notice(&raw.bytes, &raw.content_type)?;
        if inferred_encoding {
            self.event(requested, &url, "OFFICIAL_ENCODING_INFERRED", None);
        }
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

#[cfg(test)]
fn decode_page(bytes: &[u8], content_type: &str) -> Result<String, FetchError> {
    decode_page_with_notice(bytes, content_type).map(|(text, _)| text)
}

fn decode_page_with_notice(bytes: &[u8], content_type: &str) -> Result<(String, bool), FetchError> {
    if bytes.len() > MAX_BYTES {
        return Err(FetchError::new("OFFICIAL_TOO_LARGE"));
    }
    let bom = Encoding::for_bom(bytes).map(|(encoding, _)| encoding);
    let encoding = bom.or_else(|| charset(content_type)).or_else(|| {
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
    });
    let (decoded, _, malformed) = encoding.unwrap_or(UTF_8).decode(bytes);
    if malformed {
        // Legacy Chinese sites sometimes omit charset on child pages. Only infer GB18030
        // with an explicit Chinese language hint, no conflicting charset, and a lossless decode.
        let document =
            Html::parse_document(&String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]));
        let chinese = document
            .select(&Selector::parse("html[lang], meta[http-equiv]").expect("static selector"))
            .any(|node| {
                node.value()
                    .attr("lang")
                    .is_some_and(|lang| lang.to_ascii_lowercase().starts_with("zh"))
                    || (node
                        .value()
                        .attr("http-equiv")
                        .is_some_and(|value| value.eq_ignore_ascii_case("content-language"))
                        && node
                            .value()
                            .attr("content")
                            .is_some_and(|lang| lang.to_ascii_lowercase().starts_with("zh")))
            });
        if encoding.is_none() && chinese {
            let (text, _, malformed) = encoding_rs::GB18030.decode(bytes);
            if !malformed {
                return Ok((text.into_owned(), true));
            }
        }
        return Err(FetchError::new("OFFICIAL_ENCODING_FAILED"));
    }
    Ok((decoded.into_owned(), false))
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
            && !super::instruction_page_hint(node.value().attr("title").unwrap_or(""))
            && !node
                .select(&Selector::parse("img[alt]").expect("static selector"))
                .any(|image| super::instruction_page_hint(image.value().attr("alt").unwrap_or("")))
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
        async fn get(
            &self,
            url: &Url,
            _events: &mut Vec<AccessEvent>,
        ) -> Result<RawPage, FetchError> {
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
    async fn successful_requests_use_the_recorded_protocol_without_probing() {
        for scheme in ["http", "https"] {
            let input = format!("{scheme}://journal.example/guide?lang=zh");
            let mut s = session(&input, FetchOptions::default(), vec![html()]);
            let page = s.page(source_url(&input).unwrap()).await.unwrap();
            assert_eq!(page.url.as_str(), input);
            assert_eq!(*s.transport.urls.lock().unwrap(), vec![input]);
            assert_eq!(s.used_http, scheme == "http");
            assert!(s.pending.is_empty());
        }
    }

    #[tokio::test]
    async fn either_protocol_falls_back_once_without_extra_consent() {
        for (first, second) in [("http", "https"), ("https", "http")] {
            for code in [
                "OFFICIAL_CONNECTION_FAILED",
                "OFFICIAL_TLS_FAILED",
                "OFFICIAL_TIMEOUT",
                "OFFICIAL_HTTP_STATUS",
                "OFFICIAL_DNS_FAILED",
            ] {
                let input = format!("{first}://journal.example/guide?lang=zh");
                let alternate = format!("{second}://journal.example/guide?lang=zh");
                let mut s = session(
                    &input,
                    FetchOptions::default(),
                    vec![Err(FetchError::new(code)), html()],
                );
                let page = s.page(source_url(&input).unwrap()).await.unwrap();
                assert_eq!(page.url.as_str(), alternate);
                assert_eq!(
                    *s.transport.urls.lock().unwrap(),
                    vec![input.clone(), alternate]
                );
                assert!(s.pending.is_empty());
                let mut failed = session(
                    &input,
                    FetchOptions::default(),
                    vec![Err(FetchError::new(code)), Err(FetchError::new(code))],
                );
                assert!(failed.page(source_url(&input).unwrap()).await.is_err());
                assert_eq!(failed.transport.urls.lock().unwrap().len(), 2);
            }
        }
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
            vec![redirect("http://journal.example/guide"), html()],
        );
        assert_eq!(
            downgrade
                .page(source_url(seed).unwrap())
                .await
                .unwrap()
                .url
                .scheme(),
            "http"
        );
        assert!(downgrade.pending.is_empty());
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
        let legacy = format!("<meta http-equiv='Content-Language' content='zh-cn'><p>{text}</p>");
        let (bytes, _, _) = encoding_rs::GBK.encode(&legacy);
        let (decoded, inferred) = decode_page_with_notice(&bytes, "text/html").unwrap();
        assert!(inferred);
        assert!(decoded.contains(text));
        assert!(decode_page(&bytes, "text/html; charset=utf-8").is_err());
        assert!(decode_page(&[0x81], "text/html").is_err());
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
        let links = instruction_links(&base, "<a href='wltg/zgjz.htm'><font>征稿指南</font></a><a href='policy.htm'><img alt='投稿须知' src='nav.jpg'></a><a href='rules.htm' title='Author guide'></a>");
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].path(), "/wltg/zgjz.htm");
    }

    #[tokio::test]
    async fn reports_status_pdf_and_dynamic_endpoint_failure_without_hiding_them() {
        let seed = "https://journal.example/guide";
        let mut denied = session(
            seed,
            FetchOptions::default(),
            vec![
                Err(FetchError::detail("OFFICIAL_HTTP_STATUS", 403)),
                Err(FetchError::detail("OFFICIAL_HTTP_STATUS", 403)),
            ],
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
            vec![
                Err(FetchError::new("OFFICIAL_CONNECTION_FAILED")),
                Err(FetchError::new("OFFICIAL_CONNECTION_FAILED")),
            ],
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
        for (id, input) in manuscript_core::bundled_journal_homepages() {
            let seed = source_url(input).unwrap();
            let mut session =
                FetchSession::new(seed.clone(), FetchOptions::default(), PublicTransport).unwrap();
            let page = session.page(seed).await;
            let links = page
                .as_ref()
                .map(|page| instruction_links(&page.url, &page.html))
                .unwrap_or_default();
            println!(
                "{}",
                serde_json::json!({"id": id, "requestedUrl": input, "captured": page.is_ok(), "guideCandidates": links.iter().map(Url::as_str).collect::<Vec<_>>(), "events": session.events, "pending": session.pending})
            );
        }
    }

    #[test]
    fn encrypted_dns_never_allows_private_or_malformed_results() {
        let fake = "198.18.3.68:443".parse().unwrap();
        assert!(needs_encrypted_dns(&[fake]).unwrap());
        assert!(!needs_encrypted_dns(&["8.8.8.8:443".parse().unwrap()]).unwrap());
        assert!(needs_encrypted_dns(&[fake, "127.0.0.1:443".parse().unwrap()]).is_err());
        assert!(checked_addresses(vec![fake]).is_err());
        let parse = |body: serde_json::Value| dns_addresses(body.to_string().as_bytes(), 80);
        for ip in [
            "127.0.0.1",
            "198.18.2.1",
            "192.168.1.1",
            "169.254.169.254",
            "not-an-ip",
        ] {
            assert!(
                parse(serde_json::json!({"Status":0,"Answer":[{"type":1,"data":ip}]})).is_err()
            );
        }
        for body in [
            serde_json::json!({"Status":2}),
            serde_json::json!({"Status":0,"TC":true}),
            serde_json::json!({"Status":0,"Answer":[{"type":28,"data":"::ffff:127.0.0.1"}]}),
            serde_json::json!({"Status":0,"Answer":[{"type":1,"data":"8.8.8.8"},{"type":1,"data":"10.1.1.1"}]}),
        ] {
            assert!(parse(body).is_err());
        }
        assert_eq!(parse(serde_json::json!({"Status":0,"Answer":[{"type":5,"data":"alias.example"},{"type":1,"data":"8.8.8.8"}]})).unwrap(), vec!["8.8.8.8:80".parse::<SocketAddr>().unwrap()]);
        assert!(parse(serde_json::json!({"Status":0})).unwrap().is_empty());
    }

    #[test]
    fn compressed_pages_enforce_decoded_size_and_integrity() {
        use std::io::Write;
        let gzip = |text: &[u8]| {
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            encoder.write_all(text).unwrap();
            encoder.finish().unwrap()
        };
        let text = b"<p>A separate title page is required.</p>";
        assert_eq!(decompress_page(gzip(text)).unwrap(), (text.to_vec(), true));
        assert_eq!(
            decompress_page(text.to_vec()).unwrap(),
            (text.to_vec(), false)
        );
        assert_eq!(
            decompress_page(gzip(&vec![b'a'; MAX_BYTES + 1]))
                .err()
                .unwrap()
                .code,
            "OFFICIAL_TOO_LARGE"
        );
        assert_eq!(
            decompress_page(vec![0x1f, 0x8b, 0]).err().unwrap().code,
            "OFFICIAL_COMPRESSION_FAILED"
        );
    }

    #[tokio::test]
    async fn http_does_not_bypass_security_and_failures_are_not_duplicated() {
        for code in ["OFFICIAL_PRIVATE_ADDRESS", "OFFICIAL_ENCRYPTED_DNS_FAILED"] {
            let input = source_url("http://journal.example/guide").unwrap();
            let mut s = session(
                input.as_str(),
                FetchOptions::default(),
                vec![Err(FetchError::new(code))],
            );
            assert_eq!(s.page(input).await.err().unwrap().code, code);
            assert_eq!(s.transport.urls.lock().unwrap().len(), 1);
            assert_eq!(
                s.events.iter().filter(|event| event.code == code).count(),
                1
            );
            assert!(s.pending.is_empty());
        }
        let input = source_url("http://journal.example/guide").unwrap();
        let mut s = session(
            input.as_str(),
            FetchOptions::default(),
            vec![redirect("http://198.18.1.1/guide")],
        );
        assert_eq!(
            s.page(input).await.err().unwrap().code,
            "OFFICIAL_PRIVATE_ADDRESS"
        );
        assert_eq!(s.transport.urls.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn approved_domain_uses_both_protocols_but_does_not_allow_siblings() {
        let input = source_url("http://authors.publisher.example/guide").unwrap();
        let mut s = session(
            "https://journal.example",
            FetchOptions {
                approved_origins: vec!["http://authors.publisher.example".into()],
                ..Default::default()
            },
            vec![Err(FetchError::new("OFFICIAL_TLS_FAILED")), html()],
        );
        assert!(s.page(input).await.is_ok());
        assert!(s.pending.is_empty());
        assert_eq!(
            s.page(source_url("https://other.publisher.example/guide").unwrap())
                .await
                .err()
                .unwrap()
                .code,
            "OFFICIAL_ORIGIN_CONFIRMATION"
        );
    }

    #[tokio::test]
    #[ignore = "Live CJC author-guide capture with verified correction for a legacy saved URL"]
    async fn live_cjc_legacy_source_capture() {
        let recorded = "https://cjc.ict.ac.cn/";
        let correction = manuscript_core::journal_homepage_correction(recorded).unwrap();
        let seed = source_url(correction.url).unwrap();
        let mut session =
            FetchSession::new(seed.clone(), FetchOptions::default(), PublicTransport).unwrap();
        let homepage = session.page(seed).await.unwrap();
        let links = instruction_links(&homepage.url, &homepage.html);
        let mut captured = 0;
        for link in links {
            if let Ok(page) = session.page(link).await {
                println!(
                    "guide: {} ({} characters)",
                    page.url,
                    page.text.chars().count()
                );
                captured += 1;
            }
        }
        println!(
            "{}",
            serde_json::json!({"events":session.events,"pending":session.pending})
        );
        assert!(
            captured > 0,
            "must capture an actual author-guide body, not just the homepage"
        );
        assert!(session.pending.is_empty());
    }
}
