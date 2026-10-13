//! What the hosted door notes of a connection's requests as their bytes
//! arrive: whether a request carries both a `Content-Length` and a
//! `Transfer-Encoding` header.
//!
//! A request with both is the ground of request smuggling: one reader ends
//! the request where the length says and another where the chunks say. The
//! HTTP library takes the chunks, drops the length from the headers it hands
//! on, and serves the request, so the door's handler cannot see that both
//! were sent. This module watches the bytes before the library has them. It
//! follows the framing of each request far enough to find where the next one
//! starts (the length, or the chunks), and records, for each request head in
//! order, whether it named both. The door refuses such a request with 400.
//!
//! The watch reads and changes nothing. If it meets bytes it cannot frame, it
//! stops recording, and the requests after that point are served as before:
//! the library closes a connection on a request it cannot frame.

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// The longest line of a head the watch keeps. The library answers 431 to a
/// header section of this size long before.
const LONGEST_LINE: usize = 1 << 20;

/// For each request head read on one connection and not yet served, in
/// order: whether it carried both headers.
#[derive(Default)]
pub struct Framing {
    heads: Mutex<VecDeque<bool>>,
}

impl Framing {
    fn note(&self, both: bool) {
        self.heads.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push_back(both);
    }

    /// Whether the request now being served carried both headers. Called once
    /// for each request, in the order they are served, which is the order
    /// they were read. A request the watch did not record is taken not to.
    pub fn next_carries_both(&self) -> bool {
        self.heads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pop_front()
            .unwrap_or(false)
    }
}

enum Place {
    /// In a request head: the request line, then header lines, to the empty line.
    Head,
    /// In a body of a stated length, with this many bytes to go.
    Body(u64),
    /// At the line that gives the size of the next chunk.
    ChunkSize,
    /// In a chunk, with this many bytes to go.
    ChunkData(u64),
    /// At the line break that ends a chunk, with this many bytes to go.
    ChunkEnd(u8),
    /// After the last chunk: trailer lines, to the empty line.
    Trailers,
    /// The bytes could not be framed. Nothing more is recorded.
    Lost,
}

/// The framing of the requests of one connection, fed the bytes as they arrive.
pub struct HeadWatch {
    place: Place,
    line: Vec<u8>,
    request_line_read: bool,
    has_length: bool,
    has_encoding: bool,
    length: u64,
}

impl Default for HeadWatch {
    fn default() -> Self {
        HeadWatch {
            place: Place::Head,
            line: Vec::new(),
            request_line_read: false,
            has_length: false,
            has_encoding: false,
            length: 0,
        }
    }
}

impl HeadWatch {
    pub fn feed(&mut self, mut bytes: &[u8], framing: &Framing) {
        while !bytes.is_empty() {
            match self.place {
                Place::Lost => return,
                Place::Body(left) => {
                    let take = left.min(bytes.len() as u64);
                    bytes = &bytes[take as usize..];
                    self.place = if left == take { Place::Head } else { Place::Body(left - take) };
                }
                Place::ChunkData(left) => {
                    let take = left.min(bytes.len() as u64);
                    bytes = &bytes[take as usize..];
                    self.place = if left == take { Place::ChunkEnd(2) } else { Place::ChunkData(left - take) };
                }
                Place::ChunkEnd(left) => {
                    let take = (left as usize).min(bytes.len());
                    bytes = &bytes[take..];
                    self.place = if left as usize == take { Place::ChunkSize } else { Place::ChunkEnd(left - take as u8) };
                }
                Place::Head | Place::ChunkSize | Place::Trailers => {
                    let (end, whole) = match bytes.iter().position(|byte| *byte == b'\n') {
                        Some(at) => (at + 1, true),
                        None => (bytes.len(), false),
                    };
                    if self.line.len() + end > LONGEST_LINE {
                        self.place = Place::Lost;
                        return;
                    }
                    self.line.extend_from_slice(&bytes[..end]);
                    bytes = &bytes[end..];
                    if !whole {
                        return;
                    }
                    let line = std::mem::take(&mut self.line);
                    let text = line.strip_suffix(b"\n").unwrap_or(&line);
                    let text = text.strip_suffix(b"\r").unwrap_or(text);
                    self.line_read(text, framing);
                }
            }
        }
    }

    fn line_read(&mut self, text: &[u8], framing: &Framing) {
        match self.place {
            Place::Head if !self.request_line_read => {
                // Empty lines before a request line are skipped, as the library skips them.
                if !text.is_empty() {
                    self.request_line_read = true;
                }
            }
            Place::Head if text.is_empty() => {
                framing.note(self.has_length && self.has_encoding);
                // The library frames a request that names an encoding by its chunks.
                self.place = if self.has_encoding {
                    Place::ChunkSize
                } else if self.has_length && self.length > 0 {
                    Place::Body(self.length)
                } else {
                    Place::Head
                };
                self.request_line_read = false;
                self.has_length = false;
                self.has_encoding = false;
                self.length = 0;
            }
            Place::Head => {
                // A line that starts with a space or a tab continues the header before it.
                if matches!(text.first(), Some(b' ' | b'\t')) {
                    return;
                }
                let Some(colon) = text.iter().position(|byte| *byte == b':') else {
                    self.place = Place::Lost;
                    return;
                };
                let name = &text[..colon];
                if name.eq_ignore_ascii_case(b"transfer-encoding") {
                    self.has_encoding = true;
                } else if name.eq_ignore_ascii_case(b"content-length") {
                    let value = String::from_utf8_lossy(&text[colon + 1..]);
                    match value.trim().parse::<u64>() {
                        Ok(length) if value.trim().bytes().all(|byte| byte.is_ascii_digit()) => {
                            self.has_length = true;
                            self.length = length;
                        }
                        _ => self.place = Place::Lost,
                    }
                }
            }
            Place::ChunkSize => {
                let digits: Vec<u8> = text.iter().copied().take_while(u8::is_ascii_hexdigit).collect();
                let size = std::str::from_utf8(&digits).ok().and_then(|hex| u64::from_str_radix(hex, 16).ok());
                self.place = match size {
                    Some(0) => Place::Trailers,
                    Some(size) => Place::ChunkData(size),
                    None => Place::Lost,
                };
            }
            Place::Trailers => {
                if text.is_empty() {
                    self.place = Place::Head;
                }
            }
            _ => {}
        }
    }
}

/// A connection whose incoming bytes are shown to a [`HeadWatch`] on their
/// way to the HTTP library. What is read and what is written are unchanged.
pub struct Watched<S> {
    stream: S,
    watch: HeadWatch,
    framing: Arc<Framing>,
}

impl<S> Watched<S> {
    pub fn new(stream: S, framing: Arc<Framing>) -> Self {
        Watched { stream, watch: HeadWatch::default(), framing }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Watched<S> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buffer: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let before = buffer.filled().len();
        let this = &mut *self;
        let result = Pin::new(&mut this.stream).poll_read(cx, buffer);
        if let Poll::Ready(Ok(())) = &result {
            this.watch.feed(&buffer.filled()[before..], &this.framing);
        }
        result
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Watched<S> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, bytes: &[u8]) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, bytes)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        slices: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write_vectored(cx, slices)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The flags the watch records for `bytes`, fed in pieces of `piece` bytes.
    fn flags(bytes: &[u8], piece: usize) -> Vec<bool> {
        let framing = Framing::default();
        let mut watch = HeadWatch::default();
        for part in bytes.chunks(piece) {
            watch.feed(part, &framing);
        }
        let mut out = Vec::new();
        while let Some(flag) = framing.heads.lock().unwrap().pop_front() {
            out.push(flag);
        }
        out
    }

    fn every_split(bytes: &[u8], want: &[bool]) {
        for piece in [1, 2, 3, 7, 64, bytes.len()] {
            assert_eq!(flags(bytes, piece), want, "in pieces of {piece}");
        }
    }

    const BODY: &str = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}";

    fn chunked(body: &str) -> String {
        format!("{:x}\r\n{body}\r\n0\r\n\r\n", body.len())
    }

    #[test]
    fn a_request_with_a_length_or_with_chunks_alone_is_not_marked() {
        let by_length = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{BODY}", BODY.len());
        every_split(by_length.as_bytes(), &[false]);
        let by_chunks = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n{}", chunked(BODY));
        every_split(by_chunks.as_bytes(), &[false]);
        every_split(b"GET /health HTTP/1.1\r\nHost: x\r\n\r\n", &[false]);
    }

    #[test]
    fn a_request_with_both_is_marked_in_either_order_and_any_case() {
        for head in [
            format!("Transfer-Encoding: chunked\r\nContent-Length: {}\r\n", BODY.len()),
            format!("Content-Length: {}\r\nTransfer-Encoding: chunked\r\n", BODY.len()),
            format!("content-length:{}\r\nTRANSFER-ENCODING:  chunked\r\n", BODY.len()),
        ] {
            let request = format!("POST /mcp HTTP/1.1\r\nHost: x\r\n{head}\r\n{}", chunked(BODY));
            every_split(request.as_bytes(), &[true]);
        }
    }

    #[test]
    fn each_request_of_a_connection_is_marked_for_itself() {
        let clean = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{BODY}", BODY.len());
        let chunks = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n{}", chunked(BODY));
        let both = format!(
            "POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nTransfer-Encoding: chunked\r\n\r\n{}",
            BODY.len(),
            chunked(BODY)
        );
        let pipelined = format!("{clean}{chunks}\r\n{clean}{both}{clean}");
        every_split(pipelined.as_bytes(), &[false, false, false, true, false]);
    }

    #[test]
    fn header_names_inside_a_body_are_not_read_as_headers() {
        let body = "Transfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\nPOST / HTTP/1.1\r\n";
        let by_length = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}", body.len());
        every_split(by_length.as_bytes(), &[false]);
        let by_chunks = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n{}", chunked(body));
        every_split(by_chunks.as_bytes(), &[false]);
        // A trailer after the last chunk is not a header of the next request.
        let trailer = format!(
            "POST /mcp HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{BODY}\r\n0\r\nContent-Length: 3\r\n\r\nGET /health HTTP/1.1\r\nHost: x\r\n\r\n",
            BODY.len()
        );
        every_split(trailer.as_bytes(), &[false, false]);
    }

    #[test]
    fn bytes_that_cannot_be_framed_stop_the_record_and_mark_nothing() {
        every_split(b"POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: twelve\r\nTransfer-Encoding: chunked\r\n\r\n", &[]);
        every_split(b"POST /mcp HTTP/1.1\r\nno colon here\r\n\r\n", &[]);
        let framing = Framing::default();
        assert!(!framing.next_carries_both());
    }
}
