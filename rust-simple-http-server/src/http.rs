use std::fmt::{Display, Formatter};
use std::str::FromStr;

use nom::bytes::complete::{tag, take, take_until, take_while1};
use nom::character::complete::{char, multispace0, multispace1};
use nom::character::{is_alphabetic, is_alphanumeric, is_digit, is_space};
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::IResult;

use crate::http::HttpMethod::{Delete, Get, Head, Option, Post, Put};

const END_OF_INPUT: &str = "\r\n\r\n";

pub struct MediaType(pub(crate) &'static [u8]);

impl MediaType {
    pub const TEXT_PLAIN: MediaType = MediaType(b"text/plain");
    pub const APPLICATION_OCTET_STREAM: MediaType = MediaType(b"application/octet-stream");
}

impl Display for MediaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_string(self.0))
    }
}

impl From<MediaType> for &str {
    fn from(value: MediaType) -> Self {
        to_string(value.0)
    }
}

impl FromStr for MediaType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text/plain" => Ok(MediaType::TEXT_PLAIN),
            "application/octet-stream" => Ok(MediaType::APPLICATION_OCTET_STREAM),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpHeader<'a> {
    pub(crate) name: &'a [u8],
    pub(crate) value: &'a [u8],
}

impl<'a> HttpHeader<'a> {
    pub fn new(name: &'a [u8], value: &'a [u8]) -> Self {
        HttpHeader { name, value }
    }
}

impl Display for HttpHeader<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", to_string(self.name), to_string(self.value))
    }
}

pub enum HttpMethod {
    Get,
    Put,
    Post,
    Delete,
    Head,
    Option,
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Get),
            "PUT" => Ok(Put),
            "POST" => Ok(Post),
            "DELETE" => Ok(Delete),
            "HEAD" => Ok(Head),
            "OPTION" => Ok(Option),
            _ => Err(()),
        }
    }
}

pub(crate) struct HttpStatus(pub(crate) u16, &'static str, &'static str);
pub(crate) const HTTP_OK: HttpStatus = HttpStatus(200, "OK", "The request has succeeded.");
pub(crate) const HTTP_CREATED: HttpStatus = HttpStatus(
    201,
    "Created",
    "The request has been fulfilled and resulted in a new resource being created.",
);
pub(crate) const _HTTP_BAD_REQUEST: HttpStatus = HttpStatus(
    400,
    "Bad Request",
    "The request could not be understood by the server due to malformed syntax.",
);
pub(crate) const HTTP_NOT_FOUND: HttpStatus = HttpStatus(
    404,
    "Not Found",
    "The server has not found anything matching the Request-URI.",
);
pub(crate) const HTTP_METHOD_NOT_ALLOWED: HttpStatus = HttpStatus(
    405,
    "Method Not Allowed", 
    "The method specified in the Request-Line is not allowed for the resource identified by the Request-URI."
);

pub struct HttpRequest<'r> {
    pub method: HttpMethod,
    pub path: &'r str,
    pub version: &'r str,
    pub headers: Vec<HttpHeader<'r>>,
    pub body: &'r [u8],
}

pub fn parse_message(buffer: &[u8]) -> IResult<&[u8], HttpRequest> {
    let (rest, preamble) = take_until(END_OF_INPUT)(buffer)?;
    let (header_bytes, (method, path, version)) = parse_request_line(preamble)?;
    let (_should_be_empty, headers) = read_headers(header_bytes)?;
    assert!(_should_be_empty.is_empty());
    assert!(rest.len() >= END_OF_INPUT.len());
    let (body, _) = take(END_OF_INPUT.len())(rest)?;
    Ok((
        body,
        HttpRequest {
            method: HttpMethod::from_str(method).unwrap(),
            path,
            version,
            headers,
            body,
        },
    ))
}

fn parse_request_line(buffer: &[u8]) -> IResult<&[u8], (&str, &str, &str)> {
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

fn read_headers(buffer: &[u8]) -> IResult<&[u8], Vec<HttpHeader>> {
    let mut headers = Vec::new();

    // and then parse each header
    let mut rest = buffer;
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
    Ok((rest, headers))
}

fn parse_header(input: &[u8]) -> IResult<&[u8], HttpHeader> {
    let match_header_name = take_while1(|b| is_alphabetic(b) || b == b'-');
    let match_header_value = take_while1(|b| {
        is_alphanumeric(b)
            || is_space(b)
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

    let (rest, (name, value)) = match_header(input)?;

    Ok((rest, HttpHeader { name, value }))
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
    fn parse_preamble() {
        let input = b"GET /foo/bar HTTP/1.1\r\nContent-Type: text/*\r\nContent-Length: 1234\r\n\r\nblahblah";
        let res = parse_message(input);
        assert!(res.is_ok());
        let (rest, req) = res.unwrap();
        assert_eq!(b"blahblah", rest);
        assert_eq!("/foo/bar", req.path);
    }

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
        assert_eq!(b"Content-Type", header.name);
        assert_eq!(b"text/*", header.value);
    }

    #[test]
    fn test_read_headers() {
        let input = b"Content-Type: text/*\r\nContent-Length: 1234";
        let res = read_headers(input);
        assert!(res.is_ok());
        let (rest, headers) = res.unwrap();
        assert!(rest.is_empty());
        assert_eq!(2, headers.len());
        let h = &headers[0];
        assert_eq!(b"Content-Type", h.name);
        assert_eq!(b"text/*", h.value);

        let h = &headers[1];
        assert_eq!(b"Content-Length", h.name);
        assert_eq!(b"1234", h.value);
    }
}
