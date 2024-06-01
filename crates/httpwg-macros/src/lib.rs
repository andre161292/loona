//! Macros to help generate code for all suites/groups/tests of the httpwg crate

// This file is automatically @generated by httpwg-gen
// It is not intended for manual editing

/// This generates a module tree with some #[test] functions.
/// The `$body` argument is pasted inside those unit test, and
/// in that scope, `test` is the `httpwg` function you can use
/// to run the test (that takes a `mut conn: Conn<IO>`)
#[macro_export]
macro_rules! tests {
    ($body: tt) => {
        /// RFC 9113 describes an optimized expression of the
        /// semantics of the Hypertext Transfer Protocol (HTTP), referred to as
        /// HTTP version 2 (HTTP/2).
        ///
        /// HTTP/2 enables a more efficient use of network resources and a reduced
        /// latency by introducing field compression and allowing multiple concurrent
        /// exchanges on the same connection.
        ///
        /// This document obsoletes RFCs 7540 and 8740.
        ///
        /// cf. <https://httpwg.org/specs/rfc9113.html>
        #[cfg(test)]
        mod rfc9113 {
            use httpwg::rfc9113 as __suite;

            /// Section 3.4: HTTP/2 connection preface
            mod _3_4_http2_connection_preface {
                use super::__suite::_3_4_http2_connection_preface as __group;

                /// The server connection preface consists of a potentially empty
                /// SETTINGS frame (Section 6.5) that MUST be the first frame
                /// the server sends in the HTTP/2 connection.
                #[test]
                fn sends_client_connection_preface() {
                    use __group::sends_client_connection_preface as test;
                    $body
                }

                /// Clients and servers MUST treat an invalid connection preface as
                /// a connection error (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn sends_invalid_connection_preface() {
                    use __group::sends_invalid_connection_preface as test;
                    $body
                }
            }

            /// Section 4.1: Frame Format
            mod _4_1_frame_format {
                use super::__suite::_4_1_frame_format as __group;

                /// Implementations MUST ignore and discard frames of unknown types.
                #[test]
                fn sends_frame_with_unknown_type() {
                    use __group::sends_frame_with_unknown_type as test;
                    $body
                }

                /// Unused flags MUST be ignored on receipt and MUST be left
                /// unset (0x00) when sending.
                #[test]
                fn sends_frame_with_unused_flags() {
                    use __group::sends_frame_with_unused_flags as test;
                    $body
                }

                /// Reserved: A reserved 1-bit field. The semantics of this bit are
                /// undefined, and the bit MUST remain unset (0x00) when sending and
                /// MUST be ignored when receiving.
                #[test]
                fn sends_frame_with_reserved_bit_set() {
                    use __group::sends_frame_with_reserved_bit_set as test;
                    $body
                }
            }

            /// Section 4.2: Frame Size
            mod _4_2_frame_size {
                use super::__suite::_4_2_frame_size as __group;

                #[test]
                fn data_frame_with_max_length() {
                    use __group::data_frame_with_max_length as test;
                    $body
                }

                /// An endpoint MUST send an error code of FRAME_SIZE_ERROR if a frame
                /// exceeds the size defined in SETTINGS_MAX_FRAME_SIZE, exceeds any
                /// limit defined for the frame type, or is too small to contain mandatory frame data
                #[test]
                fn frame_exceeding_max_size() {
                    use __group::frame_exceeding_max_size as test;
                    $body
                }

                /// A frame size error in a frame that could alter the state of
                /// the entire connection MUST be treated as a connection error
                /// (Section 5.4.1); this includes any frame carrying a field block
                /// (Section 4.3) (that is, HEADERS, PUSH_PROMISE, and CONTINUATION),
                /// a SETTINGS frame, and any frame with a stream identifier of 0.
                #[test]
                fn large_headers_frame_exceeding_max_size() {
                    use __group::large_headers_frame_exceeding_max_size as test;
                    $body
                }
            }

            /// Section 4.3: Header Compression and Decompression
            mod _4_3_header_compression_and_decompression {
                use super::__suite::_4_3_header_compression_and_decompression as __group;

                /// A decoding error in a header block MUST be treated as a connection error
                /// (Section 5.4.1) of type COMPRESSION_ERROR.
                #[test]
                fn invalid_header_block_fragment() {
                    use __group::invalid_header_block_fragment as test;
                    $body
                }

                /// Each header block is processed as a discrete unit. Header blocks
                /// MUST be transmitted as a contiguous sequence of frames, with no
                /// interleaved frames of any other type or from any other stream.
                #[test]
                fn priority_frame_while_sending_headers() {
                    use __group::priority_frame_while_sending_headers as test;
                    $body
                }

                /// Each header block is processed as a discrete unit. Header blocks
                /// MUST be transmitted as a contiguous sequence of frames, with no
                /// interleaved frames of any other type or from any other stream.
                #[test]
                fn headers_frame_to_another_stream() {
                    use __group::headers_frame_to_another_stream as test;
                    $body
                }

                /// idle:
                /// Receiving any frame other than HEADERS or PRIORITY on a stream
                /// in this state MUST be treated as a connection error
                /// (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn idle_sends_data_frame() {
                    use __group::idle_sends_data_frame as test;
                    $body
                }

                /// idle:
                /// Receiving any frame other than HEADERS or PRIORITY on a stream
                /// in this state MUST be treated as a connection error
                /// (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn idle_sends_rst_stream_frame() {
                    use __group::idle_sends_rst_stream_frame as test;
                    $body
                }

                /// idle:
                /// Receiving any frame other than HEADERS or PRIORITY on a stream
                /// in this state MUST be treated as a connection error
                /// (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn idle_sends_window_update_frame() {
                    use __group::idle_sends_window_update_frame as test;
                    $body
                }

                /// idle:
                /// Receiving any frame other than HEADERS or PRIORITY on a stream
                /// in this state MUST be treated as a connection error
                /// (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn idle_sends_continuation_frame() {
                    use __group::idle_sends_continuation_frame as test;
                    $body
                }

                /// half-closed (remote):
                /// If an endpoint receives additional frames, other than
                /// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
                /// this state, it MUST respond with a stream error (Section 5.4.2)
                /// of type STREAM_CLOSED.
                #[test]
                fn half_closed_remote_sends_data_frame() {
                    use __group::half_closed_remote_sends_data_frame as test;
                    $body
                }

                /// half-closed (remote):
                /// If an endpoint receives additional frames, other than
                /// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
                /// this state, it MUST respond with a stream error (Section 5.4.2)
                /// of type STREAM_CLOSED.
                #[test]
                fn half_closed_remote_sends_headers_frame() {
                    use __group::half_closed_remote_sends_headers_frame as test;
                    $body
                }

                /// half-closed (remote):
                /// If an endpoint receives additional frames, other than
                /// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
                /// this state, it MUST respond with a stream error (Section 5.4.2)
                /// of type STREAM_CLOSED.
                #[test]
                fn half_closed_remote_sends_continuation_frame() {
                    use __group::half_closed_remote_sends_continuation_frame as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frame other than PRIORITY after
                /// receiving a RST_STREAM MUST treat that as a stream error
                /// (Section 5.4.2) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_data_frame_after_rst_stream() {
                    use __group::closed_sends_data_frame_after_rst_stream as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frame other than PRIORITY after
                /// receiving a RST_STREAM MUST treat that as a stream error
                /// (Section 5.4.2) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_headers_frame_after_rst_stream() {
                    use __group::closed_sends_headers_frame_after_rst_stream as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frame other than PRIORITY after
                /// receiving a RST_STREAM MUST treat that as a stream error
                /// (Section 5.4.2) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_continuation_frame_after_rst_stream() {
                    use __group::closed_sends_continuation_frame_after_rst_stream as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frames after receiving a frame
                /// with the END_STREAM flag set MUST treat that as a connection
                /// error (Section 6.4.1) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_data_frame() {
                    use __group::closed_sends_data_frame as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frames after receiving a frame
                /// with the END_STREAM flag set MUST treat that as a connection
                /// error (Section 6.4.1) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_headers_frame() {
                    use __group::closed_sends_headers_frame as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frames after receiving a frame
                /// with the END_STREAM flag set MUST treat that as a connection
                /// error (Section 6.4.1) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_continuation_frame() {
                    use __group::closed_sends_continuation_frame as test;
                    $body
                }
            }

            /// Section 5.1: Stream States
            mod _5_1_stream_states {
                use super::__suite::_5_1_stream_states as __group;

                /// idle:
                /// Receiving any frame other than HEADERS or PRIORITY on a stream
                /// in this state MUST be treated as a connection error
                /// (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn idle_sends_data_frame() {
                    use __group::idle_sends_data_frame as test;
                    $body
                }

                /// idle:
                /// Receiving any frame other than HEADERS or PRIORITY on a stream
                /// in this state MUST be treated as a connection error
                /// (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn idle_sends_rst_stream_frame() {
                    use __group::idle_sends_rst_stream_frame as test;
                    $body
                }

                /// idle:
                /// Receiving any frame other than HEADERS or PRIORITY on a stream
                /// in this state MUST be treated as a connection error
                /// (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn idle_sends_window_update_frame() {
                    use __group::idle_sends_window_update_frame as test;
                    $body
                }

                /// idle:
                /// Receiving any frame other than HEADERS or PRIORITY on a stream
                /// in this state MUST be treated as a connection error
                /// (Section 5.4.1) of type PROTOCOL_ERROR.
                #[test]
                fn idle_sends_continuation_frame() {
                    use __group::idle_sends_continuation_frame as test;
                    $body
                }

                /// half-closed (remote):
                /// If an endpoint receives additional frames, other than
                /// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
                /// this state, it MUST respond with a stream error (Section 5.4.2)
                /// of type STREAM_CLOSED.
                #[test]
                fn half_closed_remote_sends_data_frame() {
                    use __group::half_closed_remote_sends_data_frame as test;
                    $body
                }

                /// half-closed (remote):
                /// If an endpoint receives additional frames, other than
                /// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
                /// this state, it MUST respond with a stream error (Section 5.4.2)
                /// of type STREAM_CLOSED.
                #[test]
                fn half_closed_remote_sends_headers_frame() {
                    use __group::half_closed_remote_sends_headers_frame as test;
                    $body
                }

                /// half-closed (remote):
                /// If an endpoint receives additional frames, other than
                /// WINDOW_UPDATE, PRIORITY, or RST_STREAM, for a stream that is in
                /// this state, it MUST respond with a stream error (Section 5.4.2)
                /// of type STREAM_CLOSED.
                #[test]
                fn half_closed_remote_sends_continuation_frame() {
                    use __group::half_closed_remote_sends_continuation_frame as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frame other than PRIORITY after
                /// receiving a RST_STREAM MUST treat that as a stream error
                /// (Section 5.4.2) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_data_frame_after_rst_stream() {
                    use __group::closed_sends_data_frame_after_rst_stream as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frame other than PRIORITY after
                /// receiving a RST_STREAM MUST treat that as a stream error
                /// (Section 5.4.2) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_headers_frame_after_rst_stream() {
                    use __group::closed_sends_headers_frame_after_rst_stream as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frame other than PRIORITY after
                /// receiving a RST_STREAM MUST treat that as a stream error
                /// (Section 5.4.2) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_continuation_frame_after_rst_stream() {
                    use __group::closed_sends_continuation_frame_after_rst_stream as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frames after receiving a frame
                /// with the END_STREAM flag set MUST treat that as a connection
                /// error (Section 6.4.1) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_data_frame() {
                    use __group::closed_sends_data_frame as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frames after receiving a frame
                /// with the END_STREAM flag set MUST treat that as a connection
                /// error (Section 6.4.1) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_headers_frame() {
                    use __group::closed_sends_headers_frame as test;
                    $body
                }

                /// closed:
                /// An endpoint that receives any frames after receiving a frame
                /// with the END_STREAM flag set MUST treat that as a connection
                /// error (Section 6.4.1) of type STREAM_CLOSED.
                #[test]
                fn closed_sends_continuation_frame() {
                    use __group::closed_sends_continuation_frame as test;
                    $body
                }
            }
        }
    };
}
