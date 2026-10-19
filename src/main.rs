#![recursion_limit = "512"]

mod api;
mod captcha;
mod net;
mod ops;
mod parse;
mod support;
mod ui;

use anyhow::{anyhow, Result};

use net::egress::ProxyPool;
use support::cfg::Config;
use support::log::{lr, reason};

const CONFIG_PATH: &str = "config.json";
const DATA_PATH: &str = "data.txt";
const PROXY_PATH: &str = "proxy.txt";

#[tokio::main]
async fn main() {

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        println!();
        lr("Script stopped by user");
        std::process::exit(0);
    });

    #[cfg(unix)]
    tokio::spawn(async move {
        let mut term = match tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        term.recv().await;
        println!();
        lr("Script stopped by user");
        std::process::exit(0);
    });

    if let Err(error) = run().await {
        let text = error.to_string();
        if text != "#" {
            let line = if text.starts_with("File ") {
                text.clone()
            } else {
                reason(&text).to_string()
            };
            lr(&format!("The program stopped: {}", line));
        }
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let pool = ProxyPool::load_from(PROXY_PATH);
    let entry = pool.next(0);

    api::endpoints::gate(entry.as_ref()).await?;

    support::banner::print_banner();

    if let Some(proxy) = &entry {
        support::log::ly(&format!("Using proxy {}", proxy.label()));
    }

    let config = Config::load(CONFIG_PATH)?;
    let target = support::input::load(DATA_PATH)?;

    if !target.starts_with("http") {
        return Err(anyhow!("File data.txt must hold one full link"));
    }

    let mut session = ops::session::Session::connect(&config, entry.as_ref()).await?;
    ui::menu::run(&mut session).await
}
