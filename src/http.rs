//! HTTPS downloads, size-capped and limited to the hosts given, over each
//! platform's own HTTPS stack.

use std::io::{self, Read};

use crate::platform::https;

/// Whether `host` is one of `hosts` or a subdomain of one; any host when
/// `hosts` is empty.
fn within(host: &str, hosts: &[&str]) -> bool {
    hosts.is_empty()
        || hosts.iter().any(|h| {
            host.strip_suffix(h)
                .is_some_and(|sub| sub.is_empty() || sub.ends_with('.'))
        })
}

/// The body of `url`, which must be HTTPS, up to `limit` bytes.
pub fn get(url: &str, limit: usize) -> io::Result<Vec<u8>> {
    get_from(url, limit, &[], &mut |_| {})
}

/// The body of `url`, as `get`, but only from `hosts` and their subdomains,
/// wherever redirects lead. `progress` hears the bytes received so far.
pub fn get_from(url: &str, limit: usize, hosts: &[&str], progress: &mut dyn FnMut(usize)) -> io::Result<Vec<u8>> {
    let host = https::host(url)?;
    if !within(&host, hosts) {
        return Err(io::Error::other(format!("{host} is not an allowed host")));
    }
    let mut response = https::open(url)?;
    // Redirects are followed, so the host is checked again where they ended.
    if !within(response.host(), hosts) {
        return Err(io::Error::other(format!(
            "redirected to {}, which is not allowed",
            response.host()
        )));
    }
    if response.status() != 200 {
        return Err(io::Error::other(format!("HTTP {}", response.status())));
    }

    let mut body = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let read = match response.read(&mut chunk) {
            Ok(0) => return Ok(body),
            Ok(read) => read,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        body.extend_from_slice(&chunk[..read]);
        if body.len() > limit {
            return Err(io::Error::other("response too large"));
        }
        progress(body.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_only_the_hosts_given_and_their_subdomains() {
        let github = &["github.com", "githubusercontent.com"];
        assert!(within("github.com", github));
        assert!(within("release-assets.githubusercontent.com", github));
        assert!(!within("evilgithub.com", github));
        assert!(!within("github.com.evil.com", github));
        assert!(within("anything.example", &[]));
    }

    #[test]
    fn refuses_plain_http() {
        assert!(https::host("http://github.com/").is_err());
        assert_eq!(https::host("https://GitHub.com/a/b?c=1").unwrap(), "github.com");
    }
}
