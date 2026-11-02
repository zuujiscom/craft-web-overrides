use crate::R;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

/// The network, as the sync needs it; tests use a fake.
pub trait Http {
    /// GET `url` (https redirects followed) into `dest`; fails on an HTTP error or more than `max_bytes`.
    fn download(&self, url: &str, dest: &Path, timeout_secs: u64, max_bytes: u64) -> R<()>;
    /// GET `url` without following redirects: the `Location` header of a 3xx answer, if any.
    fn redirect_location(&self, url: &str, timeout_secs: u64) -> R<Option<String>>;
}

pub struct Ureq;

fn agent(timeout_secs: u64, redirects: u32) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(timeout_secs)))
        .max_redirects(redirects)
        .https_only(true)
        .http_status_as_error(redirects > 0)
        .build()
        .into()
}

impl Http for Ureq {
    fn download(&self, url: &str, dest: &Path, timeout_secs: u64, max_bytes: u64) -> R<()> {
        let resp = agent(timeout_secs, 5).get(url).call()?;
        let mut body = resp.into_body();
        let mut reader = body.as_reader().take(max_bytes + 1);
        let mut f = fs::File::create(dest)?;
        let n = std::io::copy(&mut reader, &mut f)?;
        f.flush()?;
        if n > max_bytes {
            return Err(format!("{url}: larger than {max_bytes} bytes").into());
        }
        Ok(())
    }

    fn redirect_location(&self, url: &str, timeout_secs: u64) -> R<Option<String>> {
        let resp = agent(timeout_secs, 0).get(url).call()?;
        if !resp.status().is_redirection() {
            return Ok(None);
        }
        Ok(resp.headers().get("location").and_then(|v| v.to_str().ok()).map(str::to_owned))
    }
}
