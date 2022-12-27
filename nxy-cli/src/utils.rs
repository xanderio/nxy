use serde::{Deserialize, Serialize};
use tabled::{Style, Table, Tabled};

use crate::args::Format;

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

pub(crate) fn format_output<I, T>(data: I, format: Format) -> String
where
    I: IntoIterator<Item = T> + Serialize,
    T: Tabled + Serialize,
{
    match format {
        Format::Table => Table::new(data).with(Style::rounded()).to_string(),
        //TODO: better error handling
        Format::Json => serde_json::to_string(&data).unwrap(),
    }
}

#[test]
fn format_table() {
    #[derive(Tabled, Serialize, Clone)]
    struct Foo {
        bar: String,
    }
    let data = vec![Foo {
        bar: "foobar".to_string(),
    }];

    let expected = Table::new(data.clone()).with(Style::rounded()).to_string();

    assert_eq!(format_output(data, Format::Table), expected)
}

#[test]
fn format_json() {
    #[derive(Tabled, Serialize, Clone)]
    struct Foo {
        bar: String,
    }
    let data = vec![Foo {
        bar: "foobar".to_string(),
    }];

    let expected = serde_json::to_string(&data).unwrap();

    assert_eq!(format_output(data, Format::Json), expected)
}
