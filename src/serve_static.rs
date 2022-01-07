use std::fs;

use afire::{Header, Request, Response, ServeStatic, Server};

use crate::config::{BROADCAST_ONION, DATA_DIR, ONION_SITE};
use crate::Template;
use crate::VERSION;

/// Files not to serve
const DONT_SERVE_PATH: &str = "data/config/dont_serve.txt";

lazy_static! {
    static ref DONT_SERVE: Vec<String> = {
        let mut working = Vec::new();
        let dont_serve = fs::read_to_string(DONT_SERVE_PATH).unwrap();
        for line in dont_serve.lines() {
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            working.push(line.to_owned());
        }

        working
    };
}

pub fn attach(server: &mut Server) {
    ServeStatic::new(&*DATA_DIR)
        // Inject version into reponses
        // Replaces `{{VERSION}}` with the verison in main.rs
        .middleware(|_req, res, suc| match String::from_utf8(res.data.clone()) {
            Ok(i) => Some((
                res.bytes(i.replace("{{VERSION}}", VERSION).into_bytes()),
                suc,
            )),
            Err(_) => None,
        })
        // Add Onion-Location Header
        .middleware(onion_header)
        // Response with not found if file is disabled
        .middleware(|req, _res, suc| {
            if is_disabled(&req.path).is_some() {
                return Some((not_found(&req.path), suc));
            }

            None
        })
        .not_found(|req, _| not_found(&req.path))
        .attach(server);
}

fn is_disabled(req_path: &str) -> Option<()> {
    if DONT_SERVE.contains(&req_path.to_lowercase())
        || DONT_SERVE.contains(&format!("*.{}", req_path.split('.').last()?))
    {
        return Some(());
    }
    None
}

fn not_found(path: &str) -> Response {
    Response::new()
        .status(404)
        .text(
            Template::new(
                fs::read_to_string("data/template/not_found.html")
                    .unwrap_or_else(|_| "Not Found :/".to_owned()),
            )
            .template("VERSION", VERSION)
            .template("PAGE", path)
            .build(),
        )
        .header(Header::new("Content-Type", "text/html"))
}

fn onion_header(req: Request, res: Response, suc: bool) -> Option<(Response, bool)> {
    if !*BROADCAST_ONION {
        return None;
    }

    let res = res.header(Header::new(
        "Onion-Location",
        format!("{}{}", *ONION_SITE, req.path),
    ));

    Some((res, suc))
}
