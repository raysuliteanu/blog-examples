use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;

use clap::Parser;
use itertools::Itertools;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
use tokio::net::{TcpListener, TcpStream};
use tokio::task_local;

use crate::http::{parse_message, HttpHeader, HttpMethod, HttpRequest, HttpStatus, MediaType};

mod http;

const LOCALHOST: &str = "127.0.0.1";

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

    let listener = TcpListener::bind(format!("{}:{}", LOCALHOST, port)).await?;
    let context = ThreadLocalContext::new(dir);
    loop {
        let (mut stream, _addr) = listener.accept().await?;

        tokio::spawn(CONTEXT.scope(context, async move { process_request(&mut stream).await }));
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
    port_opt.unwrap_or(8080)
}

async fn process_request(stream: &mut TcpStream) -> io::Result<()> {
    let buf = &mut Vec::<u8>::with_capacity(4096);
    loop {
        let ready = stream.ready(Interest::READABLE).await?;
        if ready.is_readable() {
            // Try to read data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match stream.try_read_buf(buf) {
                Ok(0) => break,
                Ok(_n) => {
                    // nothing to do rn
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    let (_, http_request) =
        parse_message(buf).map_err(|_e| io::Error::from(ErrorKind::InvalidData))?;

    let (cmd, args) = http::split_path(http_request.path);

    match http_request.method {
        HttpMethod::Get => match cmd {
            "" | "index.html" => handle_static_content(stream, cmd).await?,
            "files" => handle_file_download(stream, args).await?,
            "echo" => handle_echo(stream, args.as_bytes()).await?,
            "user-agent" => {
                if let Some(user_agent_header) = http_request
                    .headers
                    .iter()
                    .find(|h| h.name == b"User-Agent")
                {
                    handle_echo(stream, user_agent_header.value).await?
                } else {
                    handle_not_found(stream).await?
                }
            }
            _ => handle_not_found(stream).await?,
        },
        HttpMethod::Post => match cmd {
            "files" => handle_file_upload(stream, args, &http_request).await?,
            _ => handle_not_found(stream).await?,
        },
        _ => handle_not_implemented(stream).await?,
    }

    Ok(())
}

async fn handle_file_upload<'a>(
    stream: &mut TcpStream,
    filename: &str,
    http_request: &HttpRequest<'a>,
) -> io::Result<()> {
    let mut file = get_file(filename, true).await?;
    file.write_all(http_request.body).await?;

    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, http::HTTP_CREATED);

    build_headers(&mut buffer, vec![HttpHeader::new(b"Content-Length", b"0")]);

    build_response_body(&mut buffer, "".as_bytes());

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn handle_file_download(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    let result = read_file(filename).await;
    if let Err(e) = result {
        eprintln!("error reading {filename}: {e}");
        return handle_not_found(stream).await;
    }

    let contents = result.unwrap();

    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, http::HTTP_OK);

    let content_type = HttpHeader::new(b"Content-Type", MediaType::APPLICATION_OCTET_STREAM.0);
    let len = contents.len().to_string();
    let content_length = HttpHeader::new(b"Content-Length", len.as_bytes());

    build_headers(&mut buffer, vec![content_type, content_length]);

    build_response_body(&mut buffer, &contents[..]);

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn read_file(filename: &str) -> io::Result<Vec<u8>> {
    let mut file = get_file(filename, false).await?;
    let mut contents = vec![];
    file.read_to_end(&mut contents).await?;

    Ok(contents)
}

async fn get_file(filename: &str, create: bool) -> io::Result<File> {
    let directory = CONTEXT.get().directory.unwrap();
    let path = PathBuf::from(directory).join(filename);

    if create {
        Ok(File::create(path).await?)
    } else {
        Ok(File::open(path).await?)
    }
}

async fn handle_not_found(stream: &mut TcpStream) -> io::Result<()> {
    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, http::HTTP_NOT_FOUND);

    build_headers(&mut buffer, vec![HttpHeader::new(b"Content-Length", b"0")]);

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn handle_not_implemented(stream: &mut TcpStream) -> io::Result<()> {
    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, http::HTTP_METHOD_NOT_ALLOWED);

    build_headers(&mut buffer, vec![HttpHeader::new(b"Content-Length", b"0")]);

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn handle_static_content(stream: &mut TcpStream, _path: &str) -> io::Result<()> {
    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, http::HTTP_OK);

    build_headers(&mut buffer, vec![HttpHeader::new(b"Content-Length", b"0")]);

    build_response_body(&mut buffer, "".as_bytes());

    let _ = stream.write(&buffer).await?;

    Ok(())
}

async fn handle_echo<'a>(stream: &mut TcpStream, body: &[u8]) -> io::Result<()> {
    let mut buffer = Vec::new();
    build_protocol_header(&mut buffer, http::HTTP_OK);

    let content_type = HttpHeader::new(b"Content-Type", MediaType::TEXT_PLAIN.0);

    let len = body.len().to_string();
    let content_length = HttpHeader::new(b"Content-Length", len.as_bytes());

    build_headers(&mut buffer, vec![content_type, content_length]);

    build_response_body(&mut buffer, body);

    let _ = stream.write(&buffer).await?;

    Ok(())
}

const CR_LF: &str = "\r\n";

fn build_protocol_header(buffer: &mut Vec<u8>, http_status: HttpStatus) {
    let protocol_line = format!("HTTP/1.1 {} {}{CR_LF}", http_status.0, http_status.1);
    buffer.append(&mut protocol_line.as_bytes().to_vec());
}

fn build_headers(buffer: &mut Vec<u8>, headers: Vec<HttpHeader>) {
    let mut header_content = headers
        .iter()
        .map(|http_header: &HttpHeader| format!("{http_header}"))
        .collect_vec()
        .join(CR_LF);

    header_content.push_str(CR_LF);
    header_content.push_str(CR_LF);

    buffer.append(&mut header_content.as_bytes().to_vec())
}

fn build_response_body(buffer: &mut Vec<u8>, content: &[u8]) {
    buffer.append(&mut content.to_vec())
}
