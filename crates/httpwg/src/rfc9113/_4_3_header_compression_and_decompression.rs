//! Section 4.3: Header Compression and Decompression

use enumflags2::BitFlags;
use fluke_buffet::IntoHalves;
use fluke_h2_parse::{ContinuationFlags, HeadersFlags, PrioritySpec, StreamId};

use crate::{Conn, ErrorC};

/// A decoding error in a header block MUST be treated as a connection error
/// (Section 5.4.1) of type COMPRESSION_ERROR.
pub async fn invalid_header_block_fragment<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    conn.handshake().await?;

    // Literal Header Field with Incremental Indexing without
    // Length and String segment.
    conn.send(b"\x00\x00\x01\x01\x05\x00\x00\x00\x01\x40")
        .await?;

    conn.verify_connection_error(ErrorC::CompressionError)
        .await?;

    Ok(())
}

/// Each header block is processed as a discrete unit. Header blocks
/// MUST be transmitted as a contiguous sequence of frames, with no
/// interleaved frames of any other type or from any other stream.
pub async fn priority_frame_while_sending_headers<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    // this priority frame doesn't belong here, the peer should send
    // use a protocol error.
    conn.write_priority(
        stream_id,
        PrioritySpec {
            stream_dependency: StreamId(0),
            exclusive: false,
            weight: 255,
        },
    )
    .await?;

    let dummy_headers = conn.dummy_headers(1);
    let continuation_fragment = conn.encode_headers(&dummy_headers)?;

    // this may fail (we broke the protocol)
    _ = conn
        .write_continuation(
            stream_id,
            ContinuationFlags::EndHeaders,
            continuation_fragment,
        )
        .await;

    conn.verify_connection_error(ErrorC::ProtocolError).await?;

    Ok(())
}

/// Each header block is processed as a discrete unit. Header blocks
/// MUST be transmitted as a contiguous sequence of frames, with no
/// interleaved frames of any other type or from any other stream.
pub async fn headers_frame_to_another_stream<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, BitFlags::default(), block_fragment)
        .await?;

    // interleave a HEADERS frame for another stream
    let headers_fragment_2 = conn.encode_headers(&headers)?;
    conn.write_headers(
        StreamId(stream_id.0 + 2),
        HeadersFlags::EndHeaders,
        headers_fragment_2,
    )
    .await?;

    let dummy_headers = conn.dummy_headers(1);
    let continuation_fragment = conn.encode_headers(&dummy_headers)?;

    // this may fail (we broke the protocol)
    _ = conn
        .write_continuation(
            stream_id,
            ContinuationFlags::EndHeaders,
            continuation_fragment,
        )
        .await;

    conn.verify_connection_error(ErrorC::ProtocolError).await?;

    Ok(())
}

/// idle:
/// Receiving any frame other than HEADERS or PRIORITY on a stream
/// in this state MUST be treated as a connection error
/// (Section 5.4.1) of type PROTOCOL_ERROR.
pub async fn idle_sends_data_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    conn.handshake().await?;

    conn.write_data(StreamId(1), true, b"test").await?;

    conn.verify_connection_error(ErrorC::ProtocolError).await?;

    Ok(())
}

/// idle:
/// Receiving any frame other than HEADERS or PRIORITY on a stream
/// in this state MUST be treated as a connection error
/// (Section 5.4.1) of type PROTOCOL_ERROR.
pub async fn idle_sends_rst_stream_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    conn.handshake().await?;

    conn.write_rst_stream(StreamId(1), ErrorC::Cancel).await?;

    conn.verify_connection_error(ErrorC::ProtocolError).await?;

    Ok(())
}

/// idle:
/// Receiving any frame other than HEADERS or PRIORITY on a stream
/// in this state MUST be treated as a connection error
/// (Section 5.4.1) of type PROTOCOL_ERROR.
pub async fn idle_sends_window_update_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    conn.handshake().await?;

    conn.write_window_update(StreamId(1), 100).await?;

    conn.verify_connection_error(ErrorC::ProtocolError).await?;

    Ok(())
}

/// idle:
/// Receiving any frame other than HEADERS or PRIORITY on a stream
/// in this state MUST be treated as a connection error
/// (Section 5.4.1) of type PROTOCOL_ERROR.
pub async fn idle_sends_continuation_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_continuation(StreamId(1), ContinuationFlags::EndHeaders, block_fragment)
        .await?;

    conn.verify_connection_error(ErrorC::ProtocolError).await?;

    Ok(())
}

/// half-closed (remote):
/// If an endpoint receives additional frames, other than
/// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
/// this state, it MUST respond with a stream error (Section 5.4.2)
/// of type STREAM_CLOSED.
pub async fn half_closed_remote_sends_data_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    conn.write_data(stream_id, true, b"test").await?;

    conn.verify_stream_error(ErrorC::StreamClosed).await?;

    Ok(())
}

/// half-closed (remote):
/// If an endpoint receives additional frames, other than
/// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
/// this state, it MUST respond with a stream error (Section 5.4.2)
/// of type STREAM_CLOSED.
pub async fn half_closed_remote_sends_headers_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    conn.verify_stream_error(ErrorC::StreamClosed).await?;

    Ok(())
}

/// half-closed (remote):
/// If an endpoint receives additional frames, other than
/// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
/// this state, it MUST respond with a stream error (Section 5.4.2)
/// of type STREAM_CLOSED.
pub async fn half_closed_remote_sends_continuation_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    conn.write_continuation(stream_id, ContinuationFlags::EndHeaders, block_fragment)
        .await?;

    conn.verify_stream_error(ErrorC::StreamClosed).await?;

    Ok(())
}

/// closed:
/// An endpoint that receives any frame other than PRIORITY after
/// receiving a RST_STREAM MUST treat that as a stream error
/// (Section 5.4.2) of type STREAM_CLOSED.
pub async fn closed_sends_data_frame_after_rst_stream<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    conn.write_rst_stream(stream_id, ErrorC::Cancel).await?;

    conn.write_data(stream_id, true, b"test").await?;

    conn.verify_stream_error(ErrorC::StreamClosed).await?;

    Ok(())
}

/// closed:
/// An endpoint that receives any frame other than PRIORITY after
/// receiving a RST_STREAM MUST treat that as a stream error
/// (Section 5.4.2) of type STREAM_CLOSED.
pub async fn closed_sends_headers_frame_after_rst_stream<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    conn.write_rst_stream(stream_id, ErrorC::Cancel).await?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    conn.verify_stream_error(ErrorC::StreamClosed).await?;

    Ok(())
}

/// closed:
/// An endpoint that receives any frame other than PRIORITY after
/// receiving a RST_STREAM MUST treat that as a stream error
/// (Section 5.4.2) of type STREAM_CLOSED.
pub async fn closed_sends_continuation_frame_after_rst_stream<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndHeaders, block_fragment)
        .await?;

    conn.write_rst_stream(stream_id, ErrorC::Cancel).await?;

    let dummy_headers = conn.dummy_headers(1);
    let continuation_fragment = conn.encode_headers(&dummy_headers)?;

    conn.write_continuation(
        stream_id,
        ContinuationFlags::EndHeaders,
        continuation_fragment,
    )
    .await?;

    conn.verify_stream_error(ErrorC::StreamClosed).await?;

    Ok(())
}

/// closed:
/// An endpoint that receives any frames after receiving a frame
/// with the END_STREAM flag set MUST treat that as a connection
/// error (Section 6.4.1) of type STREAM_CLOSED.
pub async fn closed_sends_data_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndStream, block_fragment)
        .await?;

    conn.verify_stream_close(stream_id).await?;

    conn.write_data(stream_id, true, b"test").await?;

    conn.verify_stream_error(ErrorC::StreamClosed).await?;

    Ok(())
}

/// closed:
/// An endpoint that receives any frames after receiving a frame
/// with the END_STREAM flag set MUST treat that as a connection
/// error (Section 6.4.1) of type STREAM_CLOSED.
pub async fn closed_sends_headers_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndStream, block_fragment)
        .await?;

    conn.verify_stream_close(stream_id).await?;

    conn.write_headers(stream_id, HeadersFlags::EndStream, block_fragment)
        .await?;

    conn.verify_connection_error(ErrorC::StreamClosed).await?;

    Ok(())
}

/// closed:
/// An endpoint that receives any frames after receiving a frame
/// with the END_STREAM flag set MUST treat that as a connection
/// error (Section 6.4.1) of type STREAM_CLOSED.
pub async fn closed_sends_continuation_frame<IO: IntoHalves + 'static>(
    mut conn: Conn<IO>,
) -> eyre::Result<()> {
    let stream_id = StreamId(1);

    conn.handshake().await?;

    let headers = conn.common_headers();
    let block_fragment = conn.encode_headers(&headers)?;

    conn.write_headers(stream_id, HeadersFlags::EndStream, block_fragment)
        .await?;

    conn.verify_stream_close(stream_id).await?;

    let dummy_headers = conn.dummy_headers(1);
    let continuation_fragment = conn.encode_headers(&dummy_headers)?;

    conn.write_continuation(
        stream_id,
        ContinuationFlags::EndHeaders,
        continuation_fragment,
    )
    .await?;

    conn.verify_connection_error(ErrorC::StreamClosed).await?;

    Ok(())
}
