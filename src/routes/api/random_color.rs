use std::fs;

use afire::{route::RouteContext, Content, Method, Server};
use rand::seq::SliceRandom;

use crate::app::App;

const COLORS: [&str; 11] = [
    "Red", "Orange", "Yellow", "Green", "Blue", "Purple", "Pink", "Brown", "Black", "Grey", "White",
];

pub fn attach(server: &mut Server<App>) {
    let data_dir = &server.app().config.data_dir;
    let raw = fs::read_to_string(data_dir.join("colornamegen/words.txt"))
        .expect("Error Reading Words File");
    let words = raw.lines().map(|x| x.to_owned()).collect::<Vec<_>>();

    server.route(Method::GET, "/api/randomcolor", move |ctx| {
        let random_name = words
            .choose(&mut rand::thread_rng())
            .context("Error Picking Word")?;
        let random_color = COLORS
            .choose(&mut rand::thread_rng())
            .context("Error Picking Color")?;

        ctx.text(format!("{} {}", random_name, random_color))
            .content(Content::TXT)
            .send()?;
        Ok(())
    });
}
