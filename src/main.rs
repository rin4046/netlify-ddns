use anyhow::Result;
use clap::Parser;
use fetchlike::fetch_macro;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

#[derive(Parser, Debug)]
#[clap(disable_help_flag = true)]
struct Args {
    #[clap(short, long)]
    token: String,
    #[clap(short, long)]
    domain: String,
    #[clap(short, long)]
    name: String,
}

static ARGS: Lazy<Args> = Lazy::new(|| Args::parse());

#[tokio::main]
async fn main() -> Result<()> {
    let zone_id = get_zone_id().await?;
    delete_record_if_exists(&zone_id).await?;
    create_dns_record(&zone_id).await?;
    Ok(())
}

async fn get_zone_id() -> Result<String> {
    let dns_zones: Vec<DnsZone> = fetch_macro!("https://api.netlify.com/api/v1/dns_zones", {
        headers: {
            "Authorization": "Bearer ".to_string() + &ARGS.token,
            "asdf": "asdf"
        }
    })
    .await?
    .json()
    .await?;

    let zone_id = dns_zones
        .into_iter()
        .find(|x| x.name == ARGS.domain)
        .ok_or(AppError::ZoneIdNotFound(ARGS.domain.clone()))?
        .id;

    Ok(zone_id)
}

async fn delete_record_if_exists(zone_id: &str) -> Result<()> {
    let dns_records: Vec<DnsRecord> = fetch_macro!(
        format!(
            "https://api.netlify.com/api/v1/dns_zones/{}/dns_records",
            zone_id
        ),
        {
            headers: {
                "Authorization": "Bearer ".to_string() + &ARGS.token,
            },
        }
    )
    .await?
    .json()
    .await?;

    for record in dns_records {
        if record.hostname == ARGS.name.clone() + "." + &ARGS.domain && record.r#type == "A" {
            fetch_macro!(
                format!(
                    "https://api.netlify.com/api/v1/dns_zones/{}/dns_records/{}",
                    zone_id, record.id
                ),
                {
                    method: "DELETE",
                    headers: {
                        "Authorization": "Bearer ".to_string() + &ARGS.token
                    },
                }
            )
            .await?;
        }
    }

    Ok(())
}

async fn create_dns_record(zone_id: &str) -> Result<()> {
    let public_ip: PublicIp = fetch_macro!("https://httpbin.org/ip").await?.json().await?;

    fetch_macro!(format!(
            "https://api.netlify.com/api/v1/dns_zones/{}/dns_records",
            zone_id
        ),
        {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                "Authorization": "Bearer ".to_string() + &ARGS.token
            },
            body: json!({
                "type": "A",
                "hostname": ARGS.name,
                "value": public_ip.origin
            })
        }
    )
    .await?;

    println!("{}.{} -> {}", ARGS.name, ARGS.domain, public_ip.origin);

    Ok(())
}

#[derive(Deserialize)]
struct DnsZone {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct DnsRecord {
    id: String,
    hostname: String,
    r#type: String,
}

#[derive(Deserialize)]
struct PublicIp {
    origin: String,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("couldn't find dns zone: {0}")]
    ZoneIdNotFound(String),
}
