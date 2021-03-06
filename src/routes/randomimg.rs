use std::io::Read;
use std::time::Duration;

use afire::Method;
use afire::Response;
use afire::Server;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub fn attach(server: &mut Server) {
    server.route(Method::GET, "/randomimg/image.png", |_req| {
        // Try to find a ramdom image 5 times
        for _ in 1..5 {
            if let Some((i, j)) = get_random_image() {
                return Response::new()
                    .bytes(i)
                    .header("Content-Type", "image/png")
                    .header("X-Image-Id", j);
            }
        }

        Response::new().status(404).text("Error Getting Image")
    });
}

/// Try to get a random Lightshot Image
fn get_random_image() -> Option<(Vec<u8>, String)> {
    // Gen Posable Lightshot Image ID
    let chars: String = (0..6)
        .map(|_| thread_rng().sample(Alphanumeric) as char)
        .collect::<String>()
        .to_lowercase();

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(1))
        .build();

    // Get Image page
    let i = agent
        .get(&format!("https://prnt.sc/{}", chars))
        .call()
        .ok()?
        .into_string()
        .ok()?;

    // Try to extract image url
    let j = extract_image_url(i)?;

    // Check if is a screenshot that was removed
    if j == "//st.prntscr.com/2021/10/22/2139/img/0_173a7b_211be8ff.png" {
        return None;
    }

    // Load Screenshot
    let mut buf = Vec::new();
    agent
        .get(&j)
        .call()
        .ok()?
        .into_reader()
        .read_to_end(&mut buf)
        .ok()?;

    // Respond with Screenshot Bytes
    Some((buf, chars))
}

/// Extract Image Url from Lightshot Page
fn extract_image_url(body: String) -> Option<String> {
    Some(
        body.split(r#"<img class="no-click screenshot-image" src=""#)
            .nth(1)?
            .split(r#"" crossorigin="anonymous""#)
            .next()?
            .to_string(),
    )
}
