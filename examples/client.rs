//! Example client using `reqwest`.
//!
//! Usage:
//!   cargo run --example client -- login admin ChangeMe!2024
//!   cargo run --example client -- list-nodes
//!   cargo run --example client -- wol AA:BB:CC:DD:EE:FF
//!
//! Reads `GAROS_URL` (default `http://localhost:8080`) and `GAROS_TOKEN`
//! environment variables.

use reqwest::Client;
use serde_json::json;
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base = env::var("GAROS_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let mut token = env::var("GAROS_TOKEN").ok();

    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    match cmd {
        "login" => {
            let user = args.get(1).cloned().unwrap_or_else(|| "admin".into());
            let pass = args.get(2).cloned().unwrap_or_else(|| "admin".into());
            let resp = client
                .post(format!("{base}/api/auth/login"))
                .json(&json!({ "username": user, "password": pass }))
                .send()
                .await?;
            let status = resp.status();
            let body: serde_json::Value = resp.json().await?;
            println!("{} {}", status, serde_json::to_string_pretty(&body)?);
            if let Some(t) = body.get("access_token").and_then(|v| v.as_str()) {
                token = Some(t.to_string());
            }
        }
        "list-nodes" => {
            let resp = client
                .get(format!("{base}/api/garos/nodes"))
                .bearer_auth(token.as_deref().unwrap_or(""))
                .send()
                .await?;
            println!("{}", resp.text().await?);
        }
        "wol" => {
            let mac = args
                .get(1)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("mac required"))?;
            let resp = client
                .post(format!("{base}/api/garos/nodes/{mac}/wol"))
                .bearer_auth(token.as_deref().unwrap_or(""))
                .send()
                .await?;
            println!("{}", resp.text().await?);
        }
        "version" => {
            let resp = client.get(format!("{base}/version")).send().await?;
            println!("{}", resp.text().await?);
        }
        "health" => {
            let resp = client.get(format!("{base}/health")).send().await?;
            println!("{}", resp.text().await?);
        }
        _ => println!(
            "Usage: client <login|list-nodes|wol|version|health> [...]\n\
             Env: GAROS_URL, GAROS_TOKEN"
        ),
    }

    Ok(())
}
