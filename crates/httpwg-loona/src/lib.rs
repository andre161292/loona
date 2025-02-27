use b_x::{BxForResults, BX};
use std::{cell::RefCell, rc::Rc};
use tokio::{process::Command, sync::oneshot};

use buffet::{IntoHalves, RollMut};
use loona::{
    http::{self, StatusCode},
    Body, BodyChunk, Encoder, ExpectResponseHeaders, Responder, Response, ResponseDone,
    ServerDriver,
};

#[derive(Debug, Clone, Copy)]
pub enum Proto {
    H1,
    H2,
}

/// Message sent when the server is ready to accept connections.
#[derive(Debug)]
pub struct Ready {
    pub port: u16,
}

pub enum Mode {
    /// Run the server
    Server {
        ready_tx: oneshot::Sender<Ready>,
        cancel_rx: oneshot::Receiver<()>,
    },
    /// Run the server, run h2load against it, and report the results
    H2Load,
}

pub fn do_main(port: u16, proto: Proto, mode: Mode) {
    let server_start = std::time::Instant::now();

    let (ready_tx, cancel_rx, is_h2load) = match mode {
        Mode::Server {
            ready_tx,
            cancel_rx,
        } => (Some(ready_tx), Some(cancel_rx), false),
        Mode::H2Load => (None, None, true),
    };

    let server_fut = async move {
        let ln = buffet::net::TcpListener::bind(format!("127.0.0.1:{port}").parse().unwrap())
            .await
            .unwrap();
        let port = ln.local_addr().unwrap().port();

        if let Some(ready_tx) = ready_tx {
            ready_tx.send(Ready { port }).unwrap();
        }

        let child_fut = async move {
            if is_h2load {
                let mut child = Command::new("h2load")
                    .arg("-n")
                    .arg("100")
                    .arg("-c")
                    .arg("10")
                    .arg(format!("http://127.0.0.1:{}", port))
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .stdin(std::process::Stdio::null())
                    .spawn()
                    .unwrap();
                child.wait().await.unwrap();
            } else {
                // wait forever?
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        };
        let loop_fut = async move {
            let num_conns = Rc::new(RefCell::new(0));
            loop {
                let num_conns = num_conns.clone();
                tracing::debug!("Accepting...");
                let before_accept = std::time::Instant::now();
                let (stream, addr) = ln.accept().await.unwrap();

                *num_conns.borrow_mut() += 1;
                tracing::debug!(
                    ?addr,
                    "Accepted connection in {:?} ({:?} since start), total conns = {}",
                    before_accept.elapsed(),
                    server_start.elapsed(),
                    num_conns.borrow()
                );

                let conn_fut = async move {
                    struct DecrementOnDrop(Rc<RefCell<usize>>);
                    impl Drop for DecrementOnDrop {
                        fn drop(&mut self) {
                            let mut num_conns = self.0.borrow_mut();
                            *num_conns -= 1;
                        }
                    }
                    let _guard = DecrementOnDrop(num_conns);

                    let client_buf = RollMut::alloc().unwrap();
                    let io = stream.into_halves();

                    match proto {
                        Proto::H1 => {
                            let driver = TestDriver;
                            let server_conf = Rc::new(loona::h1::ServerConf {
                                ..Default::default()
                            });

                            if let Err(e) =
                                loona::h1::serve(io, server_conf, client_buf, driver).await
                            {
                                tracing::warn!("http/1 server error: {e:?}");
                            }
                            tracing::debug!("http/1 server done");
                        }
                        Proto::H2 => {
                            let driver = Rc::new(TestDriver);
                            let server_conf = Rc::new(loona::h2::ServerConf {
                                ..Default::default()
                            });

                            if let Err(e) =
                                loona::h2::serve(io, server_conf, client_buf, driver).await
                            {
                                tracing::warn!("http/2 server error: {e:?}");
                            }
                            tracing::debug!("http/2 server done");
                        }
                    }
                };

                let before_spawn = std::time::Instant::now();
                buffet::spawn(conn_fut);
                tracing::debug!("spawned connection in {:?}", before_spawn.elapsed());
            }
        };

        tokio::select! {
            _ = child_fut => {

            },
            _ = loop_fut => {},
        }
    };

    if let Some(cancel_rx) = cancel_rx {
        let cancellable_server_fut = async move {
            tokio::select! {
                _ = server_fut => {},
                _ = cancel_rx => {
                    tracing::info!("Cancelled");
                }
            }
        };

        buffet::start(cancellable_server_fut);
    } else {
        buffet::start(server_fut);
    }
}

struct TestDriver;

impl<OurEncoder> ServerDriver<OurEncoder> for TestDriver
where
    OurEncoder: Encoder,
{
    type Error = BX;

    async fn handle(
        &self,
        _req: loona::Request,
        req_body: &mut impl Body,
        mut res: Responder<OurEncoder, ExpectResponseHeaders>,
    ) -> Result<Responder<OurEncoder, ResponseDone>, Self::Error> {
        // if the client sent `expect: 100-continue`, we must send a 100 status code
        if let Some(h) = _req.headers.get(http::header::EXPECT) {
            if &h[..] == b"100-continue" {
                res.write_interim_response(Response {
                    status: StatusCode::CONTINUE,
                    ..Default::default()
                })
                .await?;
            }
        }

        // then read the full request body
        let mut req_body_len = 0;
        loop {
            let chunk = req_body.next_chunk().await.bx()?;
            match chunk {
                BodyChunk::Done { trailers } => {
                    // yey
                    if let Some(trailers) = trailers {
                        tracing::debug!(trailers_len = %trailers.len(), "received trailers");
                    }
                    break;
                }
                BodyChunk::Chunk(chunk) => {
                    req_body_len += chunk.len();
                }
            }
        }
        tracing::debug!(%req_body_len, "read request body");

        tracing::trace!("writing final response");
        let mut res = res
            .write_final_response(Response {
                status: StatusCode::OK,
                ..Default::default()
            })
            .await?;

        tracing::trace!("writing body chunk");
        res.write_chunk("it's less dire to lose, than to lose oneself".into())
            .await?;

        tracing::trace!("finishing body (with no trailers)");
        let res = res.finish_body(None).await?;

        tracing::trace!("we're done");
        Ok(res)
    }
}
