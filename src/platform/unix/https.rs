//! HTTPS through ureq and rustls, trusting Mozilla's root certificates.

use std::{io, sync::OnceLock, time::Duration};

use ureq::{Agent, BodyReader, ResponseExt, http::Uri};

const TIMEOUT: Duration = Duration::from_secs(15);
/// Long enough for the largest download over a slow link.
const TIMEOUT_ALL: Duration = Duration::from_secs(10 * 60);

fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        Agent::config_builder()
            .https_only(true)
            .http_status_as_error(false)
            .user_agent(concat!("OpenSynapse/", env!("CARGO_PKG_VERSION")))
            .timeout_connect(Some(TIMEOUT))
            .timeout_recv_response(Some(TIMEOUT))
            .timeout_global(Some(TIMEOUT_ALL))
            .build()
            .into()
    })
}

fn https_host(uri: &Uri) -> io::Result<String> {
    if uri.scheme_str() != Some("https") {
        return Err(io::Error::other("only HTTPS is allowed"));
    }
    uri.host()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| io::Error::other("no host"))
}

/// An HTTPS URL's host, in lower case.
pub fn host(url: &str) -> io::Result<String> {
    https_host(&url.parse().map_err(io::Error::other)?)
}

/// A response whose headers have arrived, its body still to read.
pub struct Response {
    status: u16,
    landed: String,
    body: BodyReader<'static>,
}

impl Response {
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The host the request ended at, after any redirects.
    pub fn host(&self) -> &str {
        &self.landed
    }
}

impl io::Read for Response {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.body.read(buf)
    }
}

/// Sends a GET for `url`, which must be HTTPS, and waits for the headers.
pub fn open(url: &str) -> io::Result<Response> {
    let response = agent().get(url).call().map_err(ureq::Error::into_io)?;
    Ok(Response {
        status: response.status().as_u16(),
        landed: https_host(response.get_uri())?,
        body: response.into_body().into_reader(),
    })
}
