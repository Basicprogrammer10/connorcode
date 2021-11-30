use std::env;
use std::time::Duration;

use afire::{Header, Response, Server};
#[macro_use]
extern crate lazy_static;

mod arg_parse;
mod routes;
mod serve_static;
mod template;
#[macro_use]
mod color;
mod analytics;
mod common;
mod config;
use color::Color;
use config::{SERVER_HOST, SERVER_PORT};
use template::Template;

pub const VERSION: &str = "5.0.0";

fn main() {
    let args: Vec<String> = env::args().collect();
    let config_file: &str =
        arg_parse::get_arg_value(&args, "--config").unwrap_or("./data/config/config.cfg");

    config::load(config_file).expect("Error Reading Config");

    println!(
        "{}",
        color::color_bold(
            format!("[*] Starting Connorcode (V{})", VERSION),
            Color::Green
        )
    );

    let mut server = Server::new(&*SERVER_HOST, *SERVER_PORT);

    server.error_handler(|_req, err| {
        Response::new()
            .status(500)
            .text(
                Template::new(include_str!("../data/template/error.html"))
                    .template("VERSION", VERSION)
                    .template("ERROR", err)
                    .build(),
            )
            .header(Header::new("Content-Type", "text/html"))
    });

    // TMP
    server.middleware(Box::new(|req| {
        println!("[{}] {}", req.method, req.path);

        None
    }));

    // Set defult headers
    server.add_default_header(Header::new("X-Content-Type-Options", "nosniff"));
    server.add_default_header(Header::new("X-Frame-Options", "DENY"));
    server.add_default_header(Header::new("X-Version", format!("Connorcode/{}", VERSION)));
    server.add_default_header(Header::new("Cache-Control", "max-age=3600"));

    // Set other things
    server.add_default_header(Header::new("X-Server", "afire/0.2.1*"));
    server.socket_timeout(Some(Duration::from_secs(1)));

    // Add my Analytics middleware
    analytics::attach(&mut server);

    // Serve Static Files
    serve_static::attach(&mut server);

    // Add Api Routes
    routes::attach(&mut server);

    color_print!(
        Color::Blue,
        "[*] Starting server on {}:{}\n",
        SERVER_HOST,
        SERVER_PORT
    );

    server.start().unwrap();
}
