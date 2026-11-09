use super::Error;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::net::UnixStream;

const LIMIT: usize = 4 * 1024 * 1024;
const LINE_LIMIT: u64 = 64 * 1024;

pub(super) struct Response {
    pub status: u16,
    reader: BufReader<UnixStream>,
    chunked: bool,
    remaining: Option<usize>,
    chunk_remaining: usize,
    ended: bool,
}

fn line(reader: &mut impl BufRead) -> Result<String, Error> {
    let mut out = String::new();
    reader.take(LINE_LIMIT).read_line(&mut out)?;
    if u64::try_from(out.len()).unwrap_or(u64::MAX) >= LINE_LIMIT {
        return Err(Error::Protocol("Header or event line exceeds limit".into()));
    }
    Ok(out)
}

impl Response {
    pub fn read(mut reader: BufReader<UnixStream>) -> Result<Self, Error> {
        let status_line = line(&mut reader)?;
        let status = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| Error::Protocol("Missing HTTP status".into()))?;
        let mut chunked = false;
        let mut remaining = None;
        let mut total = 0;
        loop {
            let header = line(&mut reader)?;
            total += header.len();
            if total > 64 * 1024 {
                return Err(Error::Protocol("Headers exceed limit".into()));
            }
            if header == "\r\n" || header == "\n" {
                break;
            }
            if header.is_empty() {
                return Err(Error::Protocol("Truncated headers".into()));
            }
            if let Some((name, value)) = header.split_once(':') {
                match name.to_ascii_lowercase().as_str() {
                    "transfer-encoding" => chunked = value.trim().eq_ignore_ascii_case("chunked"),
                    "content-length" => {
                        remaining = Some(
                            value
                                .trim()
                                .parse::<usize>()
                                .map_err(|_| Error::Protocol("Invalid Content-Length".into()))?,
                        )
                    }
                    _ => {}
                }
            }
        }
        Ok(Self {
            status,
            reader,
            chunked,
            remaining,
            chunk_remaining: 0,
            ended: false,
        })
    }

    fn byte(&mut self) -> Result<Option<u8>, Error> {
        if self.ended {
            return Ok(None);
        }
        if self.chunked && self.chunk_remaining == 0 {
            let size = line(&mut self.reader)?;
            let hex = size.trim().split(';').next().unwrap_or("");
            self.chunk_remaining = usize::from_str_radix(hex, 16)
                .map_err(|_| Error::Protocol("Invalid chunk size".into()))?;
            if self.chunk_remaining == 0 {
                self.ended = true;
                return Ok(None);
            }
            if self.chunk_remaining > LIMIT {
                return Err(Error::Protocol("Chunk exceeds limit".into()));
            }
        }
        if !self.chunked && self.remaining == Some(0) {
            self.ended = true;
            return Ok(None);
        }
        let mut byte = [0];
        if self.reader.read(&mut byte)? == 0 {
            if self.chunked || self.remaining.is_some_and(|n| n > 0) {
                return Err(Error::Protocol("Truncated response body".into()));
            }
            self.ended = true;
            return Ok(None);
        }
        if self.chunked {
            self.chunk_remaining -= 1;
            if self.chunk_remaining == 0 {
                let mut crlf = [0; 2];
                self.reader.read_exact(&mut crlf)?;
                if crlf != *b"\r\n" {
                    return Err(Error::Protocol("Invalid chunk terminator".into()));
                }
            }
        } else if let Some(remaining) = &mut self.remaining {
            *remaining -= 1;
        }
        Ok(Some(byte[0]))
    }

    pub fn body(&mut self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        while let Some(byte) = self.byte()? {
            bytes.push(byte);
            if bytes.len() > LIMIT {
                return Err(Error::Protocol("Body exceeds limit".into()));
            }
        }
        Ok(bytes)
    }
}

#[derive(Debug)]
pub struct Event {
    pub name: String,
    pub data: String,
}

pub struct Events {
    response: Response,
}

impl Events {
    pub(super) fn new(response: Response) -> Self {
        Self { response }
    }

    pub fn next_event(&mut self) -> Result<Option<Event>, Error> {
        let mut event = Event {
            name: "message".into(),
            data: String::new(),
        };
        let mut bytes = Vec::new();
        loop {
            match self.response.byte()? {
                Some(b'\n') => {
                    let text = String::from_utf8(std::mem::take(&mut bytes))
                        .map_err(|_| Error::Protocol("Event is not UTF-8".into()))?;
                    let text = text.trim_end_matches('\r');
                    if text.is_empty() && !event.data.is_empty() {
                        event.data.pop();
                        return Ok(Some(event));
                    }
                    if let Some(value) = text.strip_prefix("event:") {
                        event.name = value.trim_start().into();
                    } else if let Some(value) = text.strip_prefix("data:") {
                        event
                            .data
                            .push_str(value.strip_prefix(' ').unwrap_or(value));
                        event.data.push('\n');
                        if event.data.len() > LIMIT {
                            return Err(Error::Protocol("Event exceeds limit".into()));
                        }
                    }
                }
                Some(byte) => {
                    bytes.push(byte);
                    if bytes.len() > 64 * 1024 {
                        return Err(Error::Protocol("Event line exceeds limit".into()));
                    }
                }
                None => return Ok(None),
            }
        }
    }
}
