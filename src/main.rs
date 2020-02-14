use std::time::{Duration, Instant};
use actix::prelude::*;
use actix_files as fs;
use actix_web::{guard, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use actix_web::http::{header, Method, StatusCode};
use serde::{Deserialize, Serialize};
//use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::io;
mod schemes;
use schemes::she::hope::hope;
use schemes::she::hope::websocket::HopeWebSocket;

const HTML_FOLDER: &'static str = "static/html/";
const JS_FOLDER: &'static str = "static/js/";
const FILE_INDEX: &'static str = "index.html";
const FILE_NOTFOUND: &'static str = "404.html";

/// 404 handler
async fn p404() -> Result<fs::NamedFile, Error> {
    Ok(fs::NamedFile::open([HTML_FOLDER, FILE_NOTFOUND].concat())?.set_status_code(StatusCode::NOT_FOUND))
}

/// do websocket handshake and start `MyWebSocket` actor
async fn ws_index(r: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    println!("{:?}", r);
    let res = ws::start(HopeWebSocket::new(), &r, stream);
    println!("{:?}", res);
    res
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    // load ssl keys
    //let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    //builder
    //   .set_private_key_file("/keys/key.pem", SslFiletype::PEM)
    //    .unwrap();
    //builder.set_certificate_chain_file("/keys/cert.pem").unwrap();
        
    HttpServer::new(|| {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            //global
            .data(web::JsonConfig::default().limit(1024 * 1024)) // <- limit size of the payload (global configuration)
            // websocket route
            .service(web::resource("/router/").route(web::get().to(ws_index)))
            // static files
            .service(fs::Files::new("/", &HTML_FOLDER.to_string()).index_file(&FILE_INDEX.to_string()))
            // default
            .default_service(
                // 404 for GET request
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(HttpResponse::MethodNotAllowed),
                    ),
            )
    })
    // start http server on 127.0.0.1:8081
    //.bind_openssl("127.0.0.1:443", builder)?
    .bind("127.0.0.1:443")?
    .run()
    .await
}