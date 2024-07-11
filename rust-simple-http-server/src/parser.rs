use nom::bytes::complete::{tag, take_until, take_while1};
use nom::character::complete::{char, multispace0, multispace1};
use nom::character::{is_alphabetic, is_alphanumeric, is_digit};
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::IResult;

pub fn parse_request_line(buffer: &[u8]) -> IResult<&[u8], (&str, &str, &str)> {
    let method_parser = terminated(take_while1(is_alphabetic), multispace1);
    let path_parser = terminated(take_while1(|b| b != b' '), multispace1);
    let version_parser = take_while1(|b| is_digit(b) || b == b'.');
    let http_version_parser = terminated(preceded(tag("HTTP/"), version_parser), match_eol);

    tuple((method_parser, path_parser, http_version_parser))(buffer).map(|(rest, (m, p, v))| {
        let method = to_string(m);
        let path = to_string(p);
        let version = to_string(v);
        Ok((rest, (method, path, version)))
    })?
}

pub fn read_headers(buffer: &[u8]) -> IResult<&[u8], Vec<(&[u8], &[u8])>> {
    // read all bytes for the headers at one go
    let (r, header_bytes) = take_until("\r\n\r\n")(buffer)?;

    let mut headers = Vec::new();

    // and then parse each header
    let mut rest = header_bytes;
    while !rest.is_empty() {
        match parse_header(rest) {
            Ok((r, header)) => {
                headers.push(header);
                rest = r;
            }
            Err(e) => {
                println!("{:?}", e);
                return Err(e);
            }
        }
    }

    // r should just be the "\r\n\r\n"
    Ok((r, headers))
}

fn parse_header(input: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
    let match_header_name = take_while1(|b| is_alphabetic(b) || b == b'-');
    let match_header_value = take_while1(|b| {
        is_alphanumeric(b)
            || b == b':'
            || b == b'-'
            || b == b'/'
            || b == b'.'
            || b == b'*'
            || b == b','
            || b == b';'
            || b == b'='
    });

    let mut match_header = separated_pair(
        match_header_name,
        char(':'),
        delimited(multispace1, match_header_value, multispace0),
    );

    let header = match_header(input)?;

    Ok(header)
}

// path could be "" or "/" or "/foo" or "/foo/bar/baz" // path could be "" or "/" or "/foo" or "/foo/bar/baz"
pub fn split_path(path: &str) -> (&str, &str) {
    let try_split = path.strip_prefix('/');
    match try_split {
        Some(split) => {
            if let Some(parts) = split.split_once('/') {
                parts
            } else {
                (split, "")
            }
        }
        None => (path, ""),
    }
}

fn match_eol(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("\r\n")(input)
}

fn to_string(bytes: &[u8]) -> &str {
    unsafe { std::str::from_utf8_unchecked(bytes) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request_line() {
        let input = b"GET /foo/bar HTTP/1.1\r\n";
        let res = parse_request_line(input);
        assert!(res.is_ok());
        let (rest, (method, path, version)) = res.unwrap();
        assert!(rest.is_empty());
        assert_eq!("GET", method);
        assert_eq!("/foo/bar", path);
        assert_eq!("1.1", version);
    }

    #[test]
    fn test_parse_header() {
        let input = b"Content-Type: text/*\r\n";
        let res = parse_header(input);
        assert!(res.is_ok());
        let (rest, header) = res.unwrap();
        assert!(rest.is_empty());
        assert_eq!(b"Content-Type", header.0);
        assert_eq!(b"text/*", header.1);
    }

    #[test]
    fn test_read_headers() {
        let input = b"Content-Type: text/*\r\nContent-Length: 1234\r\n\r\n";
        let res = read_headers(input);
        assert!(res.is_ok());
        let (rest, headers) = res.unwrap();
        assert_eq!(b"\r\n\r\n", rest);
        assert_eq!(2, headers.len());
        let (h, v) = headers[0];
        assert_eq!(b"Content-Type", h);
        assert_eq!(b"text/*", v);

        let (h, v) = headers[1];
        assert_eq!(b"Content-Length", h);
        assert_eq!(b"1234", v);
    }
}
