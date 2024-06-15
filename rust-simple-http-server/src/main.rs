use std::fmt::{Display, Formatter};
use std::io;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::str::FromStr;

use clap::Parser;
use itertools::Itertools;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task_local;

const CR_LF: &str = "\r\n";
const HTTP11_PROTOCOL_STR: &str = "HTTP/1.1";
const LOCALHOST: &str = "127.0.0.1";
const READ_BUFFER_SIZE: usize = 4096;

pub struct HttpStatusCode {
    pub code: u16,
    pub mesg: &'static str,
}

impl HttpStatusCode {
    pub const HTTP_OK: HttpStatusCode = HttpStatusCode { code: 200, mesg: "OK" };
    pub const HTTP_NOT_FOUND: HttpStatusCode = HttpStatusCode { code: 404, mesg: "Not Found" };
    pub const HTTP_NOT_IMPLEMENTED: HttpStatusCode = HttpStatusCode { code: 501, mesg: "Not Implemented" };
}

impl Display for HttpStatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.code, self.mesg))
    }
}

#[derive(PartialEq, Debug)]
pub struct HttpMethod(&'static str);

impl HttpMethod {
    pub const GET: HttpMethod = HttpMethod("GET");
    pub const PUT: HttpMethod = HttpMethod("PUT");
    pub const POST: HttpMethod = HttpMethod("POST");
    pub const DELETE: HttpMethod = HttpMethod("DELETE");
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<HttpMethod, ()> {
        match s {
            "GET" => Ok(HttpMethod::GET),
            "PUT" => Ok(HttpMethod::PUT),
            "POST" => Ok(HttpMethod::POST),
            "DELETE" => Ok(HttpMethod::DELETE),
            _ => Err(())
        }
    }
}

#[derive(PartialEq)]
pub struct MediaType(&'static str);

impl MediaType {
    pub const TEXT_PLAIN: MediaType = MediaType("text/plain");
    pub const APPLICATION_OCTET_STREAM: MediaType = MediaType("application/octet-stream");
}

impl Display for MediaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<MediaType> for &str {
    fn from(value: MediaType) -> Self {
        value.0
    }
}

impl From<&str> for MediaType {
    fn from(value: &str) -> Self {
        MediaType::from_str(value).unwrap()
    }
}

impl FromStr for MediaType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text/plain" => Ok(MediaType::TEXT_PLAIN),
            "application/octet-stream" => Ok(MediaType::APPLICATION_OCTET_STREAM),
            _ => Err(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpHeader<'a> {
    name: &'a str,
    value: &'a str,
}

impl<'a> HttpHeader<'a> {
    pub fn new(name: &'a str, value: &'a str) -> Self {
        HttpHeader { name, value }
    }
}

impl Display for HttpHeader<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    directory: Option<String>,
    #[arg(short, long)]
    port: Option<u16>,
}

#[derive(Clone, Copy, Debug)]
struct ThreadLocalContext {
    directory: Option<&'static str>,
}

impl ThreadLocalContext {
    fn new(file_directory: Option<&'static str>) -> Self {
        ThreadLocalContext {
            directory: file_directory,
        }
    }
}

task_local! {
    static CONTEXT : ThreadLocalContext;
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> io::Result<()> {
    let (dir, port) = get_command_line_opts();

    let context = ThreadLocalContext::new(dir);

    let host_and_port = format!("{}:{}", LOCALHOST, port);
    println!("Listening on {host_and_port}");

    let listener = TcpListener::bind(host_and_port).await?;
    loop {
        let (mut stream, addr) = listener.accept().await?;
        println!("Connection from {addr}");

        tokio::spawn(
            CONTEXT.scope(context, async move {
                process_request(&mut stream).await
            })
        );
    }
}

fn get_command_line_opts() -> (Option<&'static str>, u16) {
    let cli = Cli::parse();

    let dir = get_directory(cli.directory);
    let port = get_port(cli.port);
    
    (dir, port)
}

fn get_directory(dir_opt: Option<String>) -> Option<&'static str> {
    if let Some(dir) = dir_opt {
        Some(Box::new(dir).leak())
    } else {
        None
    }
}

fn get_port(port_opt: Option<u16>) -> u16 {
    port_opt.unwrap_or(4221)
}

async fn process_request(stream: &mut TcpStream) -> io::Result<()> {
    let mut buf = Vec::with_capacity(READ_BUFFER_SIZE);

    read_stream(stream, &mut buf).await?;

    let lines = std::str::from_utf8(&buf)
        .map(|s| s.split(CR_LF))
        .map(|t| t.collect_vec())
        .unwrap();

    if let (Some(method), resource_path) = parse_method_line(lines.first().unwrap()) {
        let remaining_lines = lines[1..].to_vec();
        let mut headers = vec![];
        for part in remaining_lines {
            if !part.is_empty() {
                let (n, v) = part.split_once(':').unwrap();

                headers.push(HttpHeader::new(n.trim(), v.trim()));
            }
        }

        let (base_path, remaining_path_parts) = parse_path(resource_path);

        match method {
            HttpMethod::GET => {
                match base_path {
                    "" | "index.html" => handle_static_content(stream, base_path).await?,
                    "files" => handle_files(stream, remaining_path_parts).await?,
                    "echo" => handle_echo(stream, remaining_path_parts.as_bytes()).await?,
                    "user-agent" => {
                        if let Some(user_agent_header) = find_header(&headers, "User-Agent") {
                            handle_echo(stream, user_agent_header.value.as_bytes()).await?
                        } else {
                            handle_not_found(stream).await?
                        }
                    }
                    _ => handle_not_found(stream).await?,
                }
            }
            _ => handle_not_implemented(stream).await?
        }
    }

    Ok(())
}

async fn handle_files(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    let result = read_file(filename).await;
    if let Err(e) = result {
        eprintln!("error reading {filename}: {e}");
        return handle_not_found(stream).await;
    }

    let contents = result.unwrap();

    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_OK);

    let content_type = &HttpHeader::new("Content-Type", MediaType::APPLICATION_OCTET_STREAM.into());
    let len = contents.len().to_string();
    let content_length = &HttpHeader::new("Content-Length", len.as_str());

    build_headers(&mut buffer, vec![content_type, content_length]);

    build_response_body(&mut buffer, &contents[..]);

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn read_file(filename: &str) -> io::Result<Vec<u8>> {
    if let Some(directory) = CONTEXT.get().directory {
        let dir_file = PathBuf::from(directory).join(filename);
        if !dir_file.exists() {
            return Err(Error::from(ErrorKind::NotFound));
        }

        let mut file = File::open(dir_file).await?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;

        Ok(contents)
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

async fn read_stream(stream: &mut TcpStream, buf: &mut Vec<u8>) -> io::Result<()> {
    let mut total_bytes = 0usize;

    loop {
        stream.readable().await?;

        match stream.try_read_buf(buf) {
            Ok(0) => {
                break;
            }
            Ok(n) => {
                total_bytes += n;
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    buf.truncate(total_bytes);

    Ok(())
}

fn find_header<'a>(headers: &'a [HttpHeader], name: &'static str) -> Option<&'a HttpHeader<'a>> {
    headers.iter().find(|h| h.name == name)
}

// path could be "" or "/" or "/foo" or "/foo/bar/baz"
fn parse_path(path: &str) -> (&str, &str) {
    let try_split = path.strip_prefix('/');
    match try_split {
        Some(split) => {
            if let Some(parts) = split.split_once('/') {
                parts
            } else {
                (split, "")
            }
        }
        None => (path, "")
    }
}

fn parse_method_line(method_line: &str) -> (Option<HttpMethod>, &str) {
    let mut parts = method_line.split(' ');
    if let Ok(method) = HttpMethod::from_str(parts.next().unwrap()) {
        let res = (Some(method), parts.next().unwrap());
        assert_eq!(HTTP11_PROTOCOL_STR, parts.next().unwrap());
        res
    } else {
        (None, "")
    }
}

async fn handle_not_found(stream: &mut TcpStream) -> io::Result<()> {
    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_NOT_FOUND);

    let content_length: HttpHeader = HttpHeader::new("Content-Length", "0");
    build_headers(&mut buffer, vec![&content_length]);

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn handle_not_implemented(stream: &mut TcpStream) -> io::Result<()> {
    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_NOT_IMPLEMENTED);

    let content_length: HttpHeader = HttpHeader::new("Content-Length", "0");
    build_headers(&mut buffer, vec![&content_length]);

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn handle_static_content(stream: &mut TcpStream, _path: &str) -> io::Result<()> {
    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_OK);

    let content_length: HttpHeader = HttpHeader::new("Content-Length", "0");
    build_headers(&mut buffer, vec![&content_length]);

    build_response_body(&mut buffer, "".as_bytes());

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn handle_echo<'a>(stream: &mut TcpStream, body: &[u8]) -> io::Result<()> {
    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_OK);

    let content_type = &HttpHeader::new("Content-Type", MediaType::TEXT_PLAIN.into());

    let len = body.len().to_string();
    let content_length = &HttpHeader::new("Content-Length", len.as_str());

    build_headers(&mut buffer, vec![content_type, content_length]);

    build_response_body(&mut buffer, body);

    let _ = stream.write(&buffer).await?;

    Ok(())
}

fn build_protocol_header(buffer: &mut Vec<u8>, http_status_code: &HttpStatusCode) {
    let protocol_line = format!("{HTTP11_PROTOCOL_STR} {}{CR_LF}", http_status_code);
    buffer.append(&mut protocol_line.as_bytes().to_vec());
}

fn build_headers(buffer: &mut Vec<u8>, headers: Vec<&HttpHeader>) {
    let mut header_content = headers.iter()
        .map(|http_header: &&HttpHeader| {
            format!("{http_header}")
        })
        .collect_vec().join(CR_LF);

    header_content.push_str(CR_LF);
    header_content.push_str(CR_LF);

    buffer.append(&mut header_content.as_bytes().to_vec())
}

fn build_response_body(buffer: &mut Vec<u8>, content: &[u8]) {
    buffer.append(&mut content.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path() {
        let (path, args) = parse_path("");
        assert_eq!("", path);
        assert_eq!(0, args.len());

        let (path, args) = parse_path("/");
        assert_eq!("", path);
        assert_eq!(0, args.len());

        let (path, args) = parse_path("/index.html");
        assert_eq!("index.html", path);
        assert_eq!(0, args.len());

        let (path, args) = parse_path("/echo/foo/bar");
        assert_eq!("echo", path);
        assert_eq!("foo/bar", args);
    }
}
