use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use afire::extensions::RealIp;
use afire::trace;
use afire::{
    middleware::{MiddleResult, Middleware},
    Request, Server,
};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::app::App;

type Ip = String;

#[derive(Clone, Serialize, Deserialize)]
pub struct Stats {
    pub time: u64,
    pub path: String,
    pub method: Method,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
}

pub struct Analytics {
    app: Arc<App>,
    last_dump: Mutex<SystemTime>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Options,
    Head,
    Patch,
    Trace,
    Custom(String),
}

impl Middleware for Analytics {
    fn pre(&self, req: &mut Request) -> MiddleResult {
        self.save(req);
        self.check_dump();
        MiddleResult::Continue
    }

    fn attach<State>(self, server: &mut Server<State>)
    where
        Self: 'static + Send + Sync + Sized,
        State: 'static + Send + Sync,
    {
        if !self.app.config.analytics_enabled {
            return;
        }

        trace!("📦 Adding Middleware {}", type_name::<Self>());

        server.middleware.push(Box::new(self));
    }
}

impl Analytics {
    pub fn new(app: Arc<App>) -> Self {
        Analytics {
            app,
            last_dump: Mutex::new(SystemTime::now()),
        }
    }

    fn save(&self, req: &Request) -> Option<()> {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Cant get time: Time went backwards!")
            .as_secs();

        let ip = req.real_ip().to_string();
        let agent = req
            .headers
            .get("User-Agent")
            .map(|x| x.clone().into_owned());
        let referer = req.headers.get("Referer").map(|x| x.clone().into_owned());
        let stats = Stats {
            time,
            path: req.path.to_owned(),
            method: Method::from_afire(req.method),
            user_agent: agent,
            referer,
        };

        let mut this = self.app.analytics_data.lock();
        if this.contains_key(&ip) {
            let mut new = this.get(&ip)?.to_vec();
            new.push(stats);
            this.insert(ip, new);
            return Some(());
        }
        this.insert(ip, vec![stats]);
        Some(())
    }

    fn check_dump(&self) -> Option<()> {
        let mut time = self.last_dump.lock().unwrap();
        if time
            .elapsed()
            .expect("Cant get elapsed time: Time went backwards!")
            .as_secs()
            < self.app.config.dump_period
        {
            return Some(());
        }

        println!("[*] Saving Analytics");

        // Update Last Dump time
        *time = SystemTime::now();

        self.dump()?;

        Some(())
    }

    fn dump(&self) -> Option<()> {
        dump(self.app.clone())
    }
}

pub fn dump(app: Arc<App>) -> Option<()> {
    // Create Path
    let folder = Path::new(&app.config.analytics_path);
    if !folder.exists() {
        fs::create_dir_all(folder).ok()?;
    }
    let filename = Local::now().format("Analytics-%Y-%m-%d.aan").to_string();
    let path = folder.join(filename);

    // Load Prev Data
    if path.exists() {
        let old = fs::read(path.clone()).ok()?;
        let mut old = bincode::deserialize::<HashMap<Ip, Vec<Stats>>>(&old).ok()?;

        // Add New Data
        let mut data = app.analytics_data.lock();
        for i in data.clone() {
            let ip = i.0;
            let data = i.1;

            if let Some(new) = old.get(&ip) {
                let mut new = new.to_vec();
                new.extend(data);
                old.insert(ip.to_owned(), new);
                continue;
            }

            old.insert(ip.to_owned(), data);
        }

        // Reset In Memory Analytics Cache thing
        data.clear();
        let new = bincode::serialize(&old).ok()?;

        // Write New File
        fs::write(path, new).ok()?;

        return Some(());
    }

    let new = bincode::serialize(&*app.analytics_data.lock()).ok()?;
    fs::write(path, new).ok()?;
    Some(())
}

impl Method {
    fn from_afire(old: afire::Method) -> Method {
        match old {
            afire::Method::GET => Method::Get,
            afire::Method::POST => Method::Post,
            afire::Method::PUT => Method::Put,
            afire::Method::DELETE => Method::Delete,
            afire::Method::OPTIONS => Method::Options,
            afire::Method::HEAD => Method::Head,
            afire::Method::PATCH => Method::Patch,
            afire::Method::TRACE => Method::Trace,
            afire::Method::ANY => Method::Custom("ANY".to_owned()),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Delete => write!(f, "DELETE"),
            Method::Options => write!(f, "OPTIONS"),
            Method::Head => write!(f, "HEAD"),
            Method::Patch => write!(f, "PATCH"),
            Method::Trace => write!(f, "TRACE"),
            Method::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl fmt::Debug for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let out = format!(
            r#"[ ({}) {} "{}" {} {} ]"#,
            self.time,
            self.method,
            self.path,
            self.referer.clone().unwrap_or_else(|| "".to_owned()),
            self.user_agent.clone().unwrap_or_else(|| "".to_owned())
        );

        f.write_str(&out)
    }
}
