//This is the example of http request data
// GET /path/to/resource HTTP/1.1
// Host: example.com
// Accept: */*

/// Extracts the hostname from the HTTP request data.
///
/// This function takes a string slice containing HTTP request data and attempts
/// to extract the hostname from the request. It does this by iterating over the
/// lines of the HTTP data and checking if each line starts with "Host: ". If
/// it does, it extracts the hostname from the line and returns it as a `String`.
/// If no hostname is found, it returns `None`.
pub fn extract_host(http_data: &str) -> Option<String> {
    for line in http_data.lines() {
        //The line we looking for is in form of "Host: <hostname>"
        if line.to_ascii_lowercase().starts_with("Host: ") {
            return Some(line[6..].to_string());
        }
    }
    None
}

/// Extracts the path from the HTTP request data.
///
/// This function takes a string slice containing HTTP request data and attempts
/// to extract the path from the request line. The request line is assumed to be
/// the first line of the HTTP data, and the path is the second whitespace-separated
/// token in this line. If successful, it returns the path as a `String`. If the
/// path cannot be found, it returns `None`.
pub fn extract_path(http_data: &str) -> Option<String> {
    let mut lines = http_data.lines();
    if let Some(first_line) = lines.next() {
        //HTTP req usually be in form of METHOD REQUEST-URI HTTP-VERSION
        if let Some(path) = first_line.split_whitespace().nth(1) {
            return Some(path.to_string());
        }
    }
    None
}
