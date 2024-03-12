use pingora::http::RequestHeader;
use pingora::prelude::{Opt, Session};
use pingora::server::Server;
use structopt::StructOpt;
use pingora::proxy::{http_proxy_service, ProxyHttp};
use pingora_core::prelude::HttpPeer;
use log::info;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct MyGateway {
    nb_request_serve: AtomicU64,
}

#[async_trait::async_trait]
impl ProxyHttp for MyGateway {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> pingora::Result<Box<HttpPeer>> {
        Ok(Box::new(HttpPeer::new(("10.10.12.182", 880), true, "10.10.12.182".to_string())))
    }

    async fn upstream_request_filter(&self, _session: &mut Session, upstream_request: &mut RequestHeader, _ctx: &mut Self::CTX) -> pingora::Result<()> where Self::CTX: Send + Sync {
        upstream_request.insert_header("X-API-KEY", "fwd-token")?;
        upstream_request.insert_header("Content-Type", "application/json")?;
        Ok(())
    }


    async fn logging(
        &self,
        session: &mut Session,
        _e: Option<&pingora_core::Error>,
        ctx: &mut Self::CTX,
    ) {
        let response_code = session
            .response_written()
            .map_or(0, |resp| resp.status.as_u16());


        self.nb_request_serve.fetch_add(1u64, Ordering::Relaxed);

        if self.nb_request_serve.load(Ordering::Relaxed) % 10_000u64 == 0 {
            info!(
            "{} response code: {response_code}",
            self.request_summary(session, ctx)
        );
        }
    }
}

fn main() {
    env_logger::init();

    // read command line arguments
    let opt = Opt::from_args();
    let mut my_server = Server::new(Some(opt)).unwrap();
    my_server.bootstrap();

    let mut my_proxy = http_proxy_service(
        &my_server.configuration,
        MyGateway {
            nb_request_serve: AtomicU64::new(0u64),
        },
    );
    my_proxy.add_tcp("0.0.0.0:8080");
    my_server.add_service(my_proxy);

    let mut prometheus_service_http =
        pingora_core::services::listening::Service::prometheus_http_service();
    prometheus_service_http.add_tcp("127.0.0.1:8080");
    my_server.add_service(prometheus_service_http);

    my_server.run_forever();
}
