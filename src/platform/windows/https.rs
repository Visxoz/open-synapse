//! HTTPS downloads through WinHTTP, which Windows already has.

use std::{ffi::c_void, io, ptr};

use windows_sys::Win32::{
    Foundation::ERROR_INSUFFICIENT_BUFFER,
    Networking::WinHttp::{
        URL_COMPONENTS, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_INTERNET_SCHEME_HTTPS,
        WINHTTP_OPTION_URL, WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
        WinHttpCrackUrl, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpQueryOption, WinHttpReadData,
        WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
    },
};

use super::wide;

const TIMEOUT_MS: i32 = 15_000;

/// Closes a WinHTTP handle when dropped.
struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void) -> io::Result<Self> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from WinHTTP and is closed once.
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn check(ok: i32) -> io::Result<()> {
    if ok != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// An HTTPS URL's host, path with query, and port.
fn crack(url: &str) -> io::Result<(String, String, u16)> {
    let url_w = wide(url);
    let mut parts = URL_COMPONENTS {
        dwStructSize: size_of::<URL_COMPONENTS>() as u32,
        // Non-zero lengths ask for pointers into `url_w`.
        dwHostNameLength: u32::MAX,
        dwUrlPathLength: u32::MAX,
        dwExtraInfoLength: u32::MAX,
        ..Default::default()
    };
    // SAFETY: `url_w` is NUL-terminated and outlives `parts`, which points into it.
    check(unsafe { WinHttpCrackUrl(url_w.as_ptr(), 0, 0, &mut parts) })?;
    if parts.nScheme != WINHTTP_INTERNET_SCHEME_HTTPS {
        return Err(io::Error::other("only HTTPS is allowed"));
    }
    // SAFETY: the cracked pointers and lengths lie within `url_w`.
    let (host, path) = unsafe {
        (
            std::slice::from_raw_parts(parts.lpszHostName, parts.dwHostNameLength as usize),
            std::slice::from_raw_parts(
                parts.lpszUrlPath,
                (parts.dwUrlPathLength + parts.dwExtraInfoLength) as usize,
            ),
        )
    };
    Ok((
        String::from_utf16_lossy(host).to_ascii_lowercase(),
        String::from_utf16_lossy(path),
        parts.nPort,
    ))
}

/// The URL a request ended at, after any redirects.
fn final_url(request: &Handle) -> io::Result<String> {
    let mut len = 0u32;
    // SAFETY: a null buffer asks only for the length, in bytes.
    let sized = unsafe { WinHttpQueryOption(request.0, WINHTTP_OPTION_URL, ptr::null_mut(), &mut len) };
    let error = io::Error::last_os_error();
    if sized != 0 || error.raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32) {
        return Err(error);
    }
    let mut url = vec![0u16; (len as usize).div_ceil(2)];
    // SAFETY: `url` holds the `len` bytes WinHTTP asked for.
    check(unsafe { WinHttpQueryOption(request.0, WINHTTP_OPTION_URL, url.as_mut_ptr().cast(), &mut len) })?;
    url.truncate(len as usize / 2);
    Ok(String::from_utf16_lossy(&url).trim_end_matches('\0').to_owned())
}

/// An HTTPS URL's host, in lower case.
pub fn host(url: &str) -> io::Result<String> {
    crack(url).map(|(host, ..)| host)
}

/// A response whose headers have arrived, its body still to read.
pub struct Response {
    // Declared in the order they close.
    request: Handle,
    _connection: Handle,
    _session: Handle,
    status: u16,
    landed: String,
}

impl Response {
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The host the request ended at; WinHTTP follows redirects itself.
    pub fn host(&self) -> &str {
        &self.landed
    }
}

impl io::Read for Response {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut read = 0u32;
        let len = u32::try_from(buf.len()).unwrap_or(u32::MAX);
        // SAFETY: `buf` is valid for writes of `len` bytes.
        check(unsafe { WinHttpReadData(self.request.0, buf.as_mut_ptr().cast(), len, &mut read) })?;
        Ok(read as usize)
    }
}

/// Sends a GET for `url`, which must be HTTPS, and waits for the headers.
pub fn open(url: &str) -> io::Result<Response> {
    let (host, path, port) = crack(url)?;
    let agent = wide(concat!("OpenSynapse/", env!("CARGO_PKG_VERSION")));
    // SAFETY: `agent` is NUL-terminated; null proxy names use the system's.
    let session = Handle::new(unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        )
    })?;
    // SAFETY: `session` is open.
    check(unsafe { WinHttpSetTimeouts(session.0, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS) })?;
    let host_w = wide(&host);
    // SAFETY: `session` is open and `host_w` NUL-terminated.
    let connection = Handle::new(unsafe { WinHttpConnect(session.0, host_w.as_ptr(), port, 0) })?;
    let (verb, path_w) = (wide("GET"), wide(&path));
    // SAFETY: `connection` is open and the strings NUL-terminated; nulls take defaults.
    let request = Handle::new(unsafe {
        WinHttpOpenRequest(
            connection.0,
            verb.as_ptr(),
            path_w.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            WINHTTP_FLAG_SECURE,
        )
    })?;
    // SAFETY: `request` is open; no extra headers or body.
    check(unsafe { WinHttpSendRequest(request.0, ptr::null(), 0, ptr::null(), 0, 0, 0) })?;
    // SAFETY: `request` has been sent.
    check(unsafe { WinHttpReceiveResponse(request.0, ptr::null_mut()) })?;
    let (landed, ..) = crack(&final_url(&request)?)?;

    let (mut status, mut size) = (0u32, 4u32);
    // SAFETY: `status` holds the four bytes asked for.
    check(unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            (&mut status as *mut u32).cast(),
            &mut size,
            ptr::null_mut(),
        )
    })?;
    Ok(Response {
        request,
        _connection: connection,
        _session: session,
        // Codes are three digits, so one that does not fit is no answer.
        status: u16::try_from(status).unwrap_or(0),
        landed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cracks_a_url() {
        assert_eq!(
            crack("https://GitHub.com/a/b?c=1").unwrap(),
            ("github.com".into(), "/a/b?c=1".into(), 443)
        );
    }
}
