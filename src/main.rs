#![warn(clippy::pedantic)]
extern crate actix_rt;
extern crate actix_web;
extern crate futures;
extern crate mime_guess;
extern crate nfd2;
extern crate rust_embed;
extern crate web_view;

use actix_web::{body::Body, web, App, HttpRequest, HttpResponse, HttpServer};
use mime_guess::from_path;
use rust_embed::RustEmbed;
use std::{borrow::Cow, fs::File, io::prelude::*, sync::mpsc, thread};
use web_view::Content;

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
    // Channel for passing render contexts to renderer.
    let (ctx_tx, ctx_rx) = mpsc::channel();

    // Channels for passing server information to web view.
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

    let wv = web_view::builder()
        .title("Shiny!")
        .content(Content::Url(format!("http://127.0.0.1:{}", port)))
        .size(720, 640)
        .resizable(true)
        .debug(true)
        .user_data(RenderContext {
            content: "Everythings Shiny Cap'n".to_string(),
            filter: "cat".to_string(),
        })
        .invoke_handler(|wv, arg| {
            if let Some(content) = arg.strip_prefix("content:") {
                let mut ctx = wv.user_data_mut();
                ctx.content = content.to_owned();
                ctx_tx.send(ctx.clone()).unwrap();
            }
            if let Some(filter) = arg.strip_prefix("filter:") {
                let mut ctx = wv.user_data_mut();
                ctx.filter = filter.to_owned();
                ctx_tx.send(ctx.clone()).unwrap();
            }
            if let Some(dialog) = arg.strip_prefix("dialog:") {
                match dialog {
                    "load" => {
                        if let Ok(nfd2::Response::Okay(path)) = nfd2::DialogBuilder::single()
                            .filter("txt,md,rst,adoc;*")
                            .open()
                        {
                            if let Ok(mut file) = File::open(path) {
                                let mut buffer = String::new();
                                if file.read_to_string(&mut buffer).is_ok() {
                                    let ctx = wv.user_data_mut();
                                    ctx.content = buffer;
                                    ctx_tx.send(ctx.clone()).unwrap();
                                    let js = format!(
                                        "document.getElementById('editor').value = '{}'",
                                        ctx.content.escape_default().to_string()
                                    );
                                    wv.eval(&js)?;
                                } else {
                                    eprintln!("Could not read from file!");
                                }
                            } else {
                                eprintln!("Could not open file!");
                            }
                        }
                    }
                    "save" => {
                        if let Ok(nfd2::Response::Okay(path)) =
                            nfd2::DialogBuilder::new(nfd2::DialogType::SaveFile)
                                .filter("txt,md,rst,adoc;*")
                                .open()
                        {
                            let ctx = wv.user_data();
                            if let Ok(mut file) = File::create(path) {
                                if let Err(e) = file.write_all(ctx.content.as_bytes()) {
                                    eprintln!("Could not write to file! {}", e);
                                }
                            } else {
                                eprintln!("Could not save file!");
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        })
        .build()
        .unwrap();

    let hwv = wv.handle();

    // Spawna thread for rendering.
    thread::spawn(move || {
        while let Ok(ctx) = ctx_rx.recv() {
            match handlers::render(&ctx) {
                Ok(output) => {
                    let js = format!("document.getElementById('output').srcdoc = '{}'", output);
                    hwv.dispatch(move |wv| wv.eval(&js)).unwrap();
                }
                Err(e) => {
                    let js = format!("cosole.error('{}')", e);
                    hwv.dispatch(move |wv| wv.eval(&js)).unwrap();
                    eprintln!("{}", e);
                }
            }
        }
    });

    wv.run().unwrap();
    // gracefully shutdown actix web server
    server.stop(true).await;
}
