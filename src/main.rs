#![warn(clippy::pedantic)]
extern crate actix_rt;
extern crate actix_web;
extern crate futures;
extern crate mime_guess;
extern crate rust_embed;
extern crate web_view;

use actix_web::{body::Body, web, App, HttpRequest, HttpResponse, HttpServer};
use mime_guess::from_path;
use rust_embed::RustEmbed;
use std::{borrow::Cow, sync::mpsc, thread};
use web_view::{Content, WVResult, WebView};

mod handlers;

use handlers::RenderContext;

#[derive(RustEmbed)]
#[folder = "res/"]
struct Asset;

#[allow(clippy::needless_pass_by_value)]
fn assets(req: HttpRequest) -> HttpResponse {
    let path = if req.path() == "/" {
        // if there is no path, return default file
        "index.html"
    } else {
        // trim leading '/'
        &req.path()[1..]
    };

    // query the file from embedded asset with specified path
    match Asset::get(path) {
        Some(content) => {
            let body: Body = match content {
                Cow::Borrowed(bytes) => bytes.into(),
                Cow::Owned(bytes) => bytes.into(),
            };
            HttpResponse::Ok()
                .content_type(from_path(path).first_or_octet_stream().as_ref())
                .body(body)
        }
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[actix_rt::main]
async fn main() -> () {
    let (server_tx, server_rx) = mpsc::channel();
    let (port_tx, port_rx) = mpsc::channel();

    // start actix web server in separate thread
    thread::spawn(move || {
        let sys = actix_rt::System::new("shiny-server");

        let server = HttpServer::new(|| App::new().route("*", web::get().to(assets)))
            .bind("127.0.0.1:0")
            .unwrap()
            .shutdown_timeout(60);

        // we specified the port to be 0,
        // meaning the operating system
        // will choose some available port
        // for us
        // get the first bound address' port,
        // so we know where to point webview at
        let port = server.addrs().first().unwrap().port();
        let server = server.run();

        let _ = port_tx.send(port);
        let _ = server_tx.send(server);
        let _ = sys.run();
    });

    let port = port_rx.recv().unwrap();
    let server = server_rx.recv().unwrap();

    web_view::builder()
        .title("Shiny!")
        .content(Content::Url(format!("http://127.0.0.1:{}", port)))
        .size(720, 640)
        .resizable(true)
        .debug(true)
        .user_data(RenderContext {
            content: "Everythings Shiny Cap'n".to_string(),
            filter: "cat".to_string(),
        })
        .invoke_handler(invoke_handler)
        .run()
        .unwrap();

    // gracefully shutdown actix web server
    server.stop(true).await;
}

fn invoke_handler(wv: &mut WebView<RenderContext>, arg: &str) -> WVResult {
    if let Some(content) = arg.strip_prefix("content:") {
        let context = wv.user_data_mut();
        context.content = content.to_owned();
    }
    if let Some(filter) = arg.strip_prefix("filter:") {
        let context = wv.user_data_mut();
        context.filter = filter.to_owned();
    }
    handlers::render(wv);
    Ok(())
}
