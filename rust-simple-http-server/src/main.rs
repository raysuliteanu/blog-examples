use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::pin::pin;
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

#[derive(PartialEq)]
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

impl From<&[u8]> for HttpMethod {
    fn from(value: &[u8]) -> Self {
        let s = std::str::from_utf8(value);
        HttpMethod::from_str(s.unwrap()).unwrap()
    }
}

#[derive(PartialEq)]
pub struct MediaType(&'static str);

impl MediaType {
    pub const TEXT_PLAIN: MediaType = MediaType("text/plain");
    pub const APPLICATION_OCTET_STREAM: MediaType = MediaType("application/octet-stream");
}

impl Into<&'static str> for MediaType {
    fn into(self) -> &'static str {
        self.0
    }
}

impl Display for MediaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
    pub fn new(n: &'a [u8], v: &'a [u8]) -> Self {
        let name= std::str::from_utf8(n).unwrap();
        let value = std::str::from_utf8(v).unwrap();

        HttpHeader { name, value }
    }

    pub fn new_from_str(name: &'a str, value: &'a str) -> Self {
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

struct HttpRequest<'a> {
    proto: &'a str,
    raw_buf: Vec<u8>,
    io_stream: &'a mut TcpStream,
    http_method: Option<HttpMethod>,
    request_path: &'a str,
    headers: HashMap<&'a str, HttpHeader<'a>>,
}

impl<'a, 'b> HttpRequest<'a> {
    fn new(stream: &'a mut TcpStream) -> Self {
        HttpRequest {
            raw_buf: Vec::with_capacity(READ_BUFFER_SIZE),
            io_stream: stream,
            http_method: None,
            proto: HTTP11_PROTOCOL_STR,
            request_path: "/",
            headers: HashMap::default(),
        }
    }

    async fn read_line(&mut self) -> io::Result<(usize, usize)> {
        let start = self.raw_buf.len();
        let mut read = 0usize;
        loop {
            let octet = self.io_stream.read_u8().await?;
            if octet != 0x0d {
                self.raw_buf.to_vec().push(octet);
            } else {
                let possible_nl = self.io_stream.read_u8().await?;
                if possible_nl == 0x0a {
                    // got 0x0d0a which is CR_NL
                    break;
                } else {
                    // just a lone \r followed by some other byte, which seems strange but is legal I think
                    self.raw_buf.to_vec().push(possible_nl);
                }
            }

            read += 1;
        }

        Ok((start, read))
    }

    async fn parse_request_line(&mut self) -> io::Result<()> {
        let _ = self.read_line().await?;

        let mut parts = self.raw_buf.split(|b| {
            *b == ' ' as u8
        });

        let method = parts.next();
        let path = parts.next();
        let proto = parts.next();

        if method.is_none() || path.is_none() || proto.is_none() {
            todo!()
        }

        // only support HTTP/1.1
        if self.proto != std::str::from_utf8(proto.unwrap()).unwrap() {
            todo!()
        }

        let http_method = HttpMethod::from(method.unwrap());
        self.http_method = Some(http_method);
        // self.request_path = path.unwrap();

        Ok(())
    }

    // path could be "" or "/" or "/foo" or "/foo/bar/baz"
    fn parse_path(&mut self) -> (&'a str, &'a str) {
        let try_split = self.request_path.strip_prefix('/');
        match try_split {
            Some(split) => {
                if let Some(parts) = split.split_once('/') {
                    parts
                } else {
                    (split, "")
                }
            }
            None => (self.request_path, "")
        }
    }

    async fn parse_request_header(&mut self) -> io::Result<Option<HttpHeader<'a>>> {
        let (start, read) = self.read_line().await?;
        let header_line = &mut self.raw_buf[start..read];

        if read > 0 {
            if let Some(index) = header_line.into_iter().position(|b| *b == b':') {
                let (n, v) = header_line.split_at(index);
                Ok(Some(HttpHeader::new(n, v)))
            } else {
                // invalid header line, since no ':'
                todo!()
            }
        } else {
            Ok(None)
        }
    }

    async fn handle_files(&mut self, filename: &str) -> io::Result<()> {
        let result = read_file(filename).await;
        if let Err(_) = result {
            return self.handle_not_found().await;
        }

        let contents = result.unwrap();

        let mut buffer = Vec::new();
        build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_OK);

        let content_type = &HttpHeader::new_from_str("Content-Type", MediaType::APPLICATION_OCTET_STREAM.into());
        let len = contents.len().to_string();
        let content_length = &HttpHeader::new_from_str("Content-Length", len.as_str());

        build_headers(&mut buffer, vec![content_type, content_length]);

        build_response_body(&mut buffer, &contents[..]);

        self.io_stream.write(&buffer).await?;

        Ok(())
    }

    async fn handle_not_found(&mut self) -> io::Result<()> {
        let mut buffer = Vec::new();
        build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_NOT_FOUND);

        let content_length: HttpHeader = HttpHeader::new_from_str("Content-Length", "0");
        build_headers(&mut buffer, vec![&content_length]);

        self.io_stream.write(&buffer).await?;

        Ok(())
    }

    async fn handle_not_implemented(&mut self) -> io::Result<()> {
        let mut buffer = Vec::new();
        build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_NOT_IMPLEMENTED);

        let content_length: HttpHeader = HttpHeader::new_from_str("Content-Length", "0");
        build_headers(&mut buffer, vec![&content_length]);

        self.io_stream.write(&buffer).await?;

        Ok(())
    }

    async fn handle_static_content(&mut self, _path: &str) -> io::Result<()> {
        let mut buffer = Vec::new();
        build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_OK);

        let content_length: HttpHeader = HttpHeader::new_from_str("Content-Length", "0");
        build_headers(&mut buffer, vec![&content_length]);

        build_response_body(&mut buffer, "".as_bytes());

        self.io_stream.write(&buffer).await?;

        Ok(())
    }

    async fn handle_echo(&mut self, body: &[u8]) -> io::Result<()> {
        let mut buffer = Vec::new();
        build_protocol_header(&mut buffer, &HttpStatusCode::HTTP_OK);

        let content_type = &HttpHeader::new_from_str("Content-Type", MediaType::TEXT_PLAIN.into());
        let len = body.len().to_string();
        let content_length = &HttpHeader::new_from_str("Content-Length", len.as_str());

        build_headers(&mut buffer, vec![content_type, content_length]);

        build_response_body(&mut buffer, body);

        self.io_stream.write(&buffer).await?;

        Ok(())
    }

    async fn execute_request(&mut self) -> io::Result<()> {
        self.parse_request_line().await?;

        while let Some(header) = self.parse_request_header().await? {
            self.headers.insert(header.name, header);
        }

        let (base_path, remaining_path_parts) = self.parse_path();

        if let Some(m) = &mut self.http_method {
            match m {
                &mut HttpMethod::GET => {
                    match base_path {
                        "" | "index.html" => self.handle_static_content(base_path).await?,
                        "files" => self.handle_files(remaining_path_parts).await?,
                        "echo" => self.handle_echo(remaining_path_parts.as_bytes()).await?,
                        "user-agent" => {
                            if let Some(user_agent_header) = self.headers.get("User-Agent") {
                                self.handle_echo(&user_agent_header.value.as_bytes()).await?
                            } else {
                                self.handle_not_found().await?
                            }
                        }
                        _ => self.handle_not_found().await?,
                    }
                }
                &mut HttpMethod::POST => {}
                _ => self.handle_not_implemented().await?
            }
        } else {
            self.handle_not_implemented().await?
        }

        Ok(())
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> io::Result<()> {
    let (dir, port) = get_command_line_opts();

    let listener = TcpListener::bind(format!("{}:{}", LOCALHOST, port)).await?;
    let context = ThreadLocalContext::new(dir);
    loop {
        let (mut stream, _addr) = listener.accept().await?;

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
    if dir_opt.is_some() {
        Some(Box::new(dir_opt.unwrap()).leak())
    } else {
        None
    }
}

fn get_port(port_opt: Option<u16>) -> u16 {
    port_opt.or_else(|| { Some(8080) }).unwrap()
}

async fn process_request(stream: &mut TcpStream) -> io::Result<()> {
    let mut req = HttpRequest::new(stream);
    let mut request = pin!(req);
    request.execute_request().await?;

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

fn build_protocol_header(buffer: &mut Vec<u8>, http_status_code: &HttpStatusCode) {
    let protocol_line = format!("{HTTP11_PROTOCOL_STR} {}{CR_LF}", http_status_code);
    buffer.append(&mut protocol_line.as_bytes().to_vec());
}

fn build_headers(buffer: &mut Vec<u8>, headers: Vec<&HttpHeader>) {
    let mut header_content = headers.iter()
        .map(|http_header: &&HttpHeader| {
            http_header.to_string()
            // format!("{http_header}")
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
    /*    #[test]
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
*/
}
