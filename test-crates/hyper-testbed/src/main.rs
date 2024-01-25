use std::{convert::Infallible, pin::Pin};

use bytes::Bytes;
use futures::{Future, StreamExt};
use hyper::{
    service::{make_service_fn, Service},
    Body, Request, Response,
};
use tokio_stream::wrappers::ReceiverStream;
use tracing::debug;

pub(crate) struct TestService;

pub fn big_body() -> String {
    "this is a big chunk".repeat(256).repeat(128)
}

impl Service<Request<Body>> for TestService {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::pin(async move {
            let (parts, body) = req.into_parts();
            println!("Handling {parts:?}");

            let path = parts.uri.path();
            match path {
                "/echo-body" => {
                    let res = Response::builder().body(body).unwrap();
                    Ok(res)
                }
                "/stream-big-body" => {
                    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(1);
                    let rx = ReceiverStream::new(rx).map(Ok::<_, Infallible>);

                    tokio::spawn(async move {
                        let chunk = "this is a big chunk".repeat(256);
                        let chunk = Bytes::from(chunk);
                        for _ in 0..128 {
                            let _ = tx.send(chunk.clone()).await;
                        }
                    });

                    let res = Response::builder().body(Body::wrap_stream(rx)).unwrap();
                    Ok(res)
                }
                _ => {
                    let parts = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
                    if let ["status", code] = parts.as_slice() {
                        let code = code.parse::<u16>().unwrap();
                        let res = Response::builder()
                            .status(code)
                            .body(Body::empty())
                            .unwrap();
                        debug!("Replying with {res:?}");
                        Ok(res)
                    } else {
                        let res = Response::builder().status(404).body(Body::empty()).unwrap();
                        Ok(res)
                    }
                }
            }
        })
    }
}

#[tokio::main]
async fn main() {
    let upstream = hyper::Server::bind(&"127.0.0.1:0".parse().unwrap()).serve(make_service_fn(
        |_addr| async move { Ok::<_, Infallible>(TestService) },
    ));
    let upstream_addr = upstream.local_addr();
    println!("I listen on {upstream_addr}");

    upstream.await.unwrap();
}
