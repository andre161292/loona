use eyre::Context;
use futures::FutureExt;
use httparse::{Request, Response, Status, EMPTY_HEADER};
use std::{net::SocketAddr, rc::Rc};
use tokio_uring::{buf::IoBuf, net::TcpStream};
use tracing::debug;

const MAX_HEADERS_LEN: usize = 64 * 1024;
const MAX_READ_SIZE: usize = 4 * 1024;

pub use httparse;

/// re-exported so consumers can use whatever forked version we use
pub use tokio_uring;

/// A connection driver maintains per-connection state and steers requests
pub trait ConnectionDriver {
    type RequestDriver: RequestDriver;

    fn build_request_context(&self, req: &Request) -> eyre::Result<Self::RequestDriver>;
}

/// A request driver knows where a request should go, how to modify headers, etc.
pub trait RequestDriver {
    /// Determine which upstream address to use for this request
    fn upstream_addr(&self) -> eyre::Result<SocketAddr>;

    /// Returns true if this header must be kept when proxying the request upstream
    fn keep_header(&self, name: &str) -> bool;

    /// Called when extra headers should be added to the request
    fn add_extra_headers(&self, add_header: &mut dyn FnMut(&str, &[u8])) {
        // sigh
        let _ = add_header;
    }
}

/// Handle a plaintext HTTP/1.1 connection
pub async fn serve_h1(conn_dv: Rc<impl ConnectionDriver>, dos: TcpStream) -> eyre::Result<()> {
    let mut ups_rd_buf = Vec::with_capacity(MAX_READ_SIZE);
    let mut dos_rd_buf = Vec::with_capacity(MAX_READ_SIZE);

    // try to read a complete request
    let mut dos_header_buf = Vec::with_capacity(MAX_HEADERS_LEN);
    let mut ups_header_buf = Vec::with_capacity(MAX_HEADERS_LEN);

    loop {
        dos_rd_buf.clear();

        let res;
        (res, dos_rd_buf) = dos.read(dos_rd_buf).await;
        let n = res?;
        if n == 0 {
            debug!("client went away (EOF) between requests, that's fine");
            return Ok(());
        }
        let prev_dos_header_buf_len = dos_header_buf.len();
        dos_header_buf.extend_from_slice(&dos_rd_buf[..n]);

        let mut dos_headers = vec![httparse::EMPTY_HEADER; 128];
        let mut dos_req = httparse::Request::new(&mut dos_headers);
        let dos_status = dos_req
            .parse(&dos_header_buf[..])
            .wrap_err("parsing downstream request header")?;
        match dos_status {
            Status::Partial => {
                if dos_header_buf.len() >= MAX_HEADERS_LEN {
                    return Err(eyre::eyre!("downstream request headers too large"));
                } else {
                    debug!("partial request (read of size {n}), continuing to read");
                    continue;
                }
            }
            Status::Complete(req_body_offset) => {
                let req_body_offset = req_body_offset - prev_dos_header_buf_len;
                let req_body_first_part = req_body_offset..n;
                debug!(
                    "downstream request body starts with {req_body_first_part:?} into dos_read_buf"
                );

                let req_method = dos_req
                    .method
                    .ok_or_else(|| eyre::eyre!("missing method"))?;
                let req_path = dos_req
                    .path
                    .ok_or_else(|| eyre::eyre!("missing http path"))?;

                let mut connection_close = false;
                let mut req_content_length: Option<u64> = None;

                for header in dos_req.headers.iter() {
                    debug!(?header, "downstream req header");

                    if header.name.eq_ignore_ascii_case("content-length") {
                        req_content_length = Some(
                            std::str::from_utf8(header.value)
                                .wrap_err("content-length is not valid utf-8")?
                                .parse()
                                .wrap_err("could not parse content-length")?,
                        );
                    } else if header.name.eq_ignore_ascii_case("connection") {
                        #[allow(clippy::collapsible_if)]
                        if header.value.eq_ignore_ascii_case(b"close") {
                            connection_close = true;
                        }
                    } else if header.name.eq_ignore_ascii_case("transfer-encoding") {
                        if header.value.eq_ignore_ascii_case(b"chunked") {
                            return Err(eyre::eyre!("chunked transfer encoding not supported"));
                        } else {
                            return Err(eyre::eyre!(
                                "transfer-encoding not supported: {:?}",
                                std::str::from_utf8(header.value)
                            ));
                        }
                    }
                }

                let req_dv = conn_dv.build_request_context(&dos_req)?;
                let ups_addr = req_dv.upstream_addr()?;

                debug!("connecting to upstream at {ups_addr}...");
                let ups = TcpStream::connect(ups_addr).await?;

                debug!("writing request header to upstream...");
                ups_header_buf.clear();
                ups_header_buf.extend_from_slice(req_method.as_bytes());
                ups_header_buf.push(b' ');
                ups_header_buf.extend_from_slice(req_path.as_bytes());
                ups_header_buf.push(b' ');
                ups_header_buf.extend_from_slice(b"HTTP/1.1\r\n");

                for header in dos_req.headers.iter() {
                    if !req_dv.keep_header(header.name) {
                        continue;
                    }

                    ups_header_buf.extend_from_slice(header.name.as_bytes());
                    ups_header_buf.extend_from_slice(b": ");
                    ups_header_buf.extend_from_slice(header.value);
                    ups_header_buf.extend_from_slice(b"\r\n");
                }

                req_dv.add_extra_headers(&mut |name, value| {
                    ups_header_buf.extend_from_slice(name.as_bytes());
                    ups_header_buf.extend_from_slice(b": ");
                    ups_header_buf.extend_from_slice(value);
                    ups_header_buf.extend_from_slice(b"\r\n");
                });

                ups_header_buf.extend_from_slice(b"\r\n");

                let res;
                (res, ups_header_buf) = ups.write_all(ups_header_buf).await;
                res.wrap_err("writing request header upstream")?;

                let mut res_content_length: Option<u64> = None;
                let res_body_first_part;

                debug!("reading response header from upstream...");
                'read_response: loop {
                    ups_header_buf.clear();

                    ups_rd_buf.clear();
                    let res;
                    (res, ups_rd_buf) = ups.read(ups_rd_buf).await;
                    let n = res?;
                    if n == 0 {
                        return Err(eyre::eyre!("app closed connection before sending response"));
                    }
                    let prev_ups_header_buf_len = ups_header_buf.len();
                    ups_header_buf.extend_from_slice(&ups_rd_buf[..n]);

                    let mut ups_headers = vec![EMPTY_HEADER; 128];
                    let mut ups_res = Response::new(&mut ups_headers);
                    let ups_status = ups_res
                        .parse(&ups_header_buf[..])
                        .wrap_err("parsing downstream response header")?;
                    match ups_status {
                        Status::Partial => {
                            if ups_header_buf.len() >= MAX_HEADERS_LEN {
                                return Err(eyre::eyre!("upstream response headers too large"));
                            } else {
                                debug!("partial response (read of size {n}), continuing to read");
                                continue;
                            }
                        }
                        Status::Complete(res_body_offset) => {
                            let res_body_offset = res_body_offset - prev_ups_header_buf_len;
                            res_body_first_part = res_body_offset..n;
                            debug!("upstream response body starts with {res_body_first_part:?} into ups_read_buf");

                            debug!("writing response header to downstream...");

                            let res_status = ups_res
                                .code
                                .ok_or_else(|| eyre::eyre!("missing http status"))?;

                            dos_header_buf.clear();
                            dos_header_buf.extend_from_slice(b"HTTP/1.1 ");
                            dos_header_buf.extend_from_slice(format!("{} ", res_status).as_bytes());
                            if let Some(reason) = ups_res.reason {
                                dos_header_buf.extend_from_slice(reason.as_bytes());
                            }
                            dos_header_buf.extend_from_slice(b"\r\n");

                            for header in ups_res.headers.iter() {
                                debug!(?header, "upstream res header");

                                if header.name.eq_ignore_ascii_case("content-length") {
                                    res_content_length = Some(
                                        std::str::from_utf8(header.value)
                                            .wrap_err("content-length is not valid utf-8")?
                                            .parse()
                                            .wrap_err("could not parse content-length")?,
                                    );
                                }

                                dos_header_buf.extend_from_slice(header.name.as_bytes());
                                dos_header_buf.extend_from_slice(b": ");
                                dos_header_buf.extend_from_slice(header.value);
                                dos_header_buf.extend_from_slice(b"\r\n");
                            }

                            dos_header_buf.extend_from_slice(b"\r\n");

                            let res;
                            (res, dos_header_buf) = dos.write_all(dos_header_buf).await;
                            res.wrap_err("writing response header to downstream")?;

                            break 'read_response;
                        }
                    }
                }

                let mut req_content_length = req_content_length.unwrap_or(0);
                let mut res_content_length = res_content_length.unwrap_or(0);

                debug!("writing first request body part to upstream ({req_body_first_part:?})...");
                req_content_length -= req_body_first_part.len() as u64;
                let res;
                let slice;
                (res, slice) = ups.write_all(dos_rd_buf.slice(req_body_first_part)).await;
                res.wrap_err("writing request body (first chunk) to upstream")?;
                dos_rd_buf = slice.into_inner();
                debug!("{req_content_length} req body left");

                debug!(
                    "writing first response body part to downstream ({res_body_first_part:?})..."
                );
                res_content_length -= res_body_first_part.len() as u64;
                let res;
                let slice;
                (res, slice) = dos.write_all(ups_rd_buf.slice(res_body_first_part)).await;
                res.wrap_err("writing response body (first chunk) to downstream")?;
                ups_rd_buf = slice.into_inner();
                debug!("{res_content_length} res body left");

                debug!("proxying bodies in both directions");

                let req_body_fut =
                    copy("req body", &dos, &ups, dos_rd_buf, &mut req_content_length)
                        .map(|r| r.wrap_err("copying request body to upstream"));
                let res_body_fut =
                    copy("res body", &ups, &dos, ups_rd_buf, &mut res_content_length)
                        .map(|r| r.wrap_err("copying response body to downstream"));

                (dos_rd_buf, ups_rd_buf) = tokio::try_join!(res_body_fut, req_body_fut)?;

                if connection_close {
                    debug!("client requested connection close");
                    return Ok(());
                }

                dos_header_buf.clear();
            }
        };
    }
}

// Copy `content_length` bytes from `src` to `dst` using the provided buffer.
async fn copy(
    role: &str,
    src: &TcpStream,
    dst: &TcpStream,
    buf: Vec<u8>,
    content_length: &mut u64,
) -> eyre::Result<Vec<u8>> {
    let mut buf = buf;

    while *content_length > 0 {
        debug!(%role, "{content_length} left");
        buf.clear();

        let res;
        (res, buf) = src.read(buf).await;
        let n = res.wrap_err("reading")?;
        *content_length -= n as u64;
        debug!(%role, "read {n} bytes, {content_length} left");

        if n == 0 {
            debug!(%role, "end of stream");
            return Ok(buf);
        }

        let res;
        let slice;
        (res, slice) = dst.write_all(buf.slice(..n)).await;
        res.wrap_err("writing")?;
        buf = slice.into_inner();
        debug!(%role, "wrote {n} bytes, {content_length} left");
    }

    Ok(buf)
}
