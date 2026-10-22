use anyhow::{anyhow, Result};
use std::time::Duration;

use crate::api::{endpoints, protocol};
use crate::captcha::solver;
use crate::net::egress::ProxyEntry;
use crate::net::{client::Http, fingerprint};
use crate::parse::Dashboard;
use crate::support::cfg::Config;
use crate::support::log::unit_word;
use crate::support::log::{countdown, lg, ly, reason};

const GATE_ATTEMPTS: usize = 8;
const PROBE_SECONDS: u64 = 8;

async fn reachable(proxy: &ProxyEntry) -> bool {
    let socket = tokio::net::TcpStream::connect((proxy.host.as_str(), proxy.port));
    matches!(
        tokio::time::timeout(Duration::from_secs(PROBE_SECONDS), socket).await,
        Ok(Ok(_))
    )
}

pub struct Session {
    pub http: Http,
    pub sleep_seconds: u64,
    pub dashboard: Dashboard,
    entry: Option<ProxyEntry>,
}

impl Session {
    pub async fn connect(config: &Config, entry: Option<&ProxyEntry>) -> Result<Self> {
        let mut entry = entry;

        if let Some(proxy) = entry {
            if !reachable(proxy).await {
                ly(&format!("The proxy {} did not answer", proxy.label()));
                ly("Continuing without a proxy");
                entry = None;
            }
        }

        let mut session = Self::build(config, entry.cloned()).await?;

        let failure = match session.authenticate().await {
            Ok(()) => return Ok(session),
            Err(error) => error,
        };

        let Some(proxy) = entry else {
            return Err(failure);
        };

        ly(&format!("The proxy {} dropped the session", proxy.label()));
        ly("Continuing without a proxy");

        let mut direct = Self::build(config, None).await?;
        direct.authenticate().await?;
        Ok(direct)
    }

    async fn build(config: &Config, entry: Option<ProxyEntry>) -> Result<Self> {
        let mut http = Http::new(entry.as_ref())?;
        http.capture_identity().await?;
        Ok(Self {
            http,
            sleep_seconds: config.sleep_seconds() as u64,
            dashboard: Dashboard {
                cards: Vec::new(),
                panels: Vec::new(),
            },
            entry,
        })
    }

    pub async fn relogin(&mut self) -> Result<()> {
        let mut http = Http::new(self.entry.as_ref())?;
        http.capture_identity().await?;
        self.http = http;
        self.dashboard = Dashboard {
            cards: Vec::new(),
            panels: Vec::new(),
        };
        self.sign_in(false).await
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        self.sign_in(true).await
    }

    pub async fn restart(&mut self) -> Result<()> {
        self.sign_in(false).await
    }

    async fn sign_in(&mut self, announce: bool) -> Result<()> {
        let (_, page) = self.http.get_text(&endpoints::home()).await?;

        if protocol::is_dashboard(&page) {
            self.dashboard = Dashboard::parse(&page);
            if announce {
                let total = self.dashboard.cards.len() as u64;
                lg(&format!(
                    "{} {} listed",
                    total,
                    unit_word(total, "service is", "services are")
                ));
            }
            return Ok(());
        }

        ly("Captcha gate is up, reading the challenge");

        for attempt in 1..=GATE_ATTEMPTS {
            match self.attempt_gate().await {
                Ok(Some(page)) => {
                    self.dashboard = Dashboard::parse(&page);
                    lg(&format!("Gate cleared on attempt {}", attempt));
                    if announce {
                        let total = self.dashboard.cards.len() as u64;
                        lg(&format!(
                            "{} {} listed",
                            total,
                            unit_word(total, "service is", "services are")
                        ));
                    }
                    return Ok(());
                }
                Ok(None) => ly(&format!(
                    "Captcha answer rejected, attempt {} of {}",
                    attempt, GATE_ATTEMPTS
                )),
                Err(error) => {
                    if error.to_string().contains("Cloudflare") {
                        return Err(error);
                    }
                    ly(&format!(
                        "Attempt {} failed: {}, trying again",
                        attempt,
                        reason(&error.to_string())
                    ))
                }
            }
            tokio::time::sleep(Duration::from_millis(700)).await;
        }

        Err(anyhow!("The captcha gate was not cleared"))
    }

    pub async fn refresh(&mut self) -> Result<bool> {
        let (_, page) = self.http.get_text(&endpoints::home()).await?;
        if !protocol::is_dashboard(&page) {
            return Ok(false);
        }
        self.dashboard = Dashboard::parse(&page);
        Ok(true)
    }

    pub async fn ensure(&mut self) -> Result<()> {
        if self.refresh().await? {
            return Ok(());
        }
        self.restart().await
    }

    async fn attempt_gate(&self) -> Result<Option<String>> {
        let stamp = chrono::Utc::now().timestamp();
        let lookup = self.http.get_xhr(&endpoints::captcha_lookup(stamp)).await?;

        if lookup.contains("Just a moment") || lookup.contains("cf-browser-verification") {
            return Err(anyhow!(
                "Cloudflare is challenging this address, change the proxy or wait it out"
            ));
        }

        let path = protocol::unwrap_captcha_path(&lookup)
            .ok_or_else(|| anyhow!("The challenge lookup shape changed"))?;

        let image = self.http.get_bytes(&endpoints::absolute(&path)).await?;
        let answer = solver::read(&image)?;

        let report = fingerprint::build(&self.http.user_agent, &self.http.language);
        let sealed = protocol::seal(&report.to_string());

        let form = vec![
            ("captchalogin".to_string(), answer),
            ("captcha_encoded".to_string(), sealed),
        ];
        self.http.post_form(&endpoints::home(), &form).await?;

        let (_, page) = self.http.get_text(&endpoints::home()).await?;
        if protocol::is_dashboard(&page) {
            Ok(Some(page))
        } else {
            Ok(None)
        }
    }

    #[cfg(test)]
    pub fn for_test(http: Http, sleep_seconds: u64, dashboard: Dashboard) -> Self {
        Self {
            http,
            sleep_seconds,
            dashboard,
            entry: None,
        }
    }

    pub async fn sleep_server(&self, seconds: u64) {
        countdown(seconds, "Retry in").await;
    }

    pub async fn sleep_cycle(&self) {
        countdown(self.sleep_seconds, "Next cycle starts in").await;
        crate::support::banner::print_banner();
    }
}
