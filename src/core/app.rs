use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use console::style;
use hyper::body::Bytes;
use hyper::{body, service::service_fn, Request, Response};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use std::convert::Infallible;
use tokio::sync::mpsc::Sender;
use tokio::{net::TcpListener, sync::Mutex};

use super::config::Config;
use super::constant::APP_NAME;
use super::logger::init_logger;
use super::server::handle;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

pub struct App {
    config: Config,
    addr: SocketAddr,
    listener: TcpListener,
}

impl App {
    pub async fn new(config_path: &str, spawn_tx: Option<Sender<String>>) -> Self {
        let _ = init_logger(spawn_tx);

        let config = Config::new(&config_path);

        let addr = config
            .listen_address()
            .to_socket_addrs()
            .expect("invalid listend address or port")
            .next()
            .expect("failed to resolve address");
        let listener = TcpListener::bind(addr)
            .await
            .expect("tcp listener failed to bind address");

        App {
            config,
            addr,
            listener,
        }
    }

    pub async fn start(&self) {
        println!(
            "\nGreetings from {APP_NAME} !!\nListening on {} ...\n",
            style(format!("http://{}", self.addr)).cyan()
        );
        let app_state = Arc::new(Mutex::new(self.config.clone()));
        loop {
            let (stream, _) = self
                .listener
                .accept()
                .await
                .expect("tcp listener failed to accept");
            let io = TokioIo::new(stream);

            let app_state = app_state.clone();
            tokio::task::spawn(async move {
                // Finally, we bind the incoming connection to our `hello` service
                if let Err(err) = Builder::new(TokioExecutor::new())
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(
                        io,
                        service_fn(move |req: Request<body::Incoming>| {
                            service(req, app_state.clone())
                        }),
                    )
                    .await
                {
                    eprintln!("error serving connection: {:?}", err);
                }
            });
        }
    }

    /// start hyper http server
    #[deprecated]
    pub async fn start_server(
        config_path: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("\nGreetings from {APP_NAME} !!\n");

        let config = Config::new(&config_path);

        let addr = config
            .listen_address()
            .to_socket_addrs()
            .expect("invalid listend address or port")
            .next()
            .expect("failed to resolve address");
        println!(
            "\nListening on {} ...\n",
            style(format!("http://{}", &addr)).cyan()
        );
        let app_state = Arc::new(Mutex::new(config.clone()));
        let listener = TcpListener::bind(addr)
            .await
            .expect("tcp listener failed to bind address");
        loop {
            let (stream, _) = listener
                .accept()
                .await
                .expect("tcp listener failed to accept");
            let io = TokioIo::new(stream);

            let app_state = app_state.clone();
            tokio::task::spawn(async move {
                // Finally, we bind the incoming connection to our `hello` service
                if let Err(err) = Builder::new(TokioExecutor::new())
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(
                        io,
                        service_fn(move |req: Request<body::Incoming>| {
                            service(req, app_state.clone())
                        }),
                    )
                    .await
                {
                    eprintln!("error serving connection: {:?}", err);
                }
            });
        }
    }
}

/// handle http service
async fn service(
    req: Request<body::Incoming>,
    app_state: Arc<Mutex<Config>>,
) -> Result<Response<BoxBody>, hyper::http::Error> {
    handle(req, Arc::clone(&app_state)).await
}
