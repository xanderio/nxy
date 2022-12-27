/// Returns value of the `NXY_SERVER` enviorment variable or `http://localhost:8080`
#[must_use]
pub(crate) fn server_url() -> String {
    std::env::var("NXY_SERVER").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Prefix an request path with the server url.
///
/// # Arguments
///
/// * `path` - URL path to be prefixed. eg. `/api/v1/flakes`
///
/// # Returns
///
/// Path prefixed with server url
///
/// # Notes
///
/// The server url is provided by [`server_url`].
#[must_use]
pub(crate) fn format_url(path: &str) -> String {
    let host = server_url();
    let path = path.strip_prefix('/').unwrap_or(path);
    format!("{host}/{path}")
}
