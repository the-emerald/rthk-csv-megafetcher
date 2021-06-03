use crate::schema::Format::{Audio, Video};
use crate::schema::Language::{Chinese, English};
use crate::schema::{Entry, Format, Language};
use anyhow::anyhow;
use clap::{load_yaml, App};
use futures::TryFutureExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Response;
use std::fs::{create_dir_all, File};
use std::io::{copy, BufReader};
use std::path::Path;
use std::str::FromStr;

pub mod schema;

pub const OUTPUT: &str = "./output";
pub const RETRIES: usize = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let yaml = load_yaml!("cli.yml");
    let args = App::from_yaml(yaml).get_matches();

    // Configs
    let source = BufReader::new(File::open(args.value_of("source").unwrap())?);
    let language = args
        .value_of("language")
        .map(|lang| Language::from_str(lang))
        .transpose()?
        .map(|lang| vec![lang])
        .unwrap_or_else(|| vec![English, Chinese]);
    let format = args
        .value_of("format")
        .map(|fmt| Format::from_str(fmt))
        .transpose()?
        .map(|fmt| vec![fmt])
        .unwrap_or_else(|| vec![Audio, Video]);

    let mut to_fetch = Vec::new();
    let mut reader = csv::Reader::from_reader(source);
    let csv_reading_pbar = {
        let pb = ProgressBar::new_spinner().with_style(
            ProgressStyle::default_spinner().template("{prefix:.bold.dim} {spinner} {wide_msg}"),
        );
        pb.set_message("Reading manifest");
        pb.set_prefix("[1/6]");
        pb
    };

    for line in reader.deserialize() {
        let record: Entry = line?;
        to_fetch.push(record);
    }

    csv_reading_pbar.finish();

    // Filter
    let to_fetch = to_fetch
        .into_iter()
        .filter(|entry| language.contains(&entry.language))
        .filter(|entry| format.contains(&entry.format))
        .collect::<Vec<_>>();

    // Set up what we need for fetching
    let semaphore = tokio::sync::Semaphore::new(32);
    let client = reqwest::Client::new();
    create_dir_all(Path::new(OUTPUT))?;

    let download_pbar = {
        let pb = ProgressBar::new(to_fetch.len() as u64).with_style(
            ProgressStyle::default_bar()
                .template("{prefix:.bold.dim} {spinner} {msg} [{elapsed_precise}] [{wide_bar}] {pos}/{len} ({eta})")
        );
        pb.set_message("Attempt 1");
        pb.set_prefix("[2/6]");
        pb
    };

    let fetched = futures::future::join_all(to_fetch.into_iter().map(|entry| async {
        // Get semaphore
        let _permit = semaphore.acquire().await?;

        // Reqwest from client
        let r = client
            .get(entry.file_url.clone())
            .send()
            .map_err(|e| anyhow!(e))
            .and_then(|response| write_to_file(response, entry))
            .await;
        download_pbar.inc(1);
        r
    }))
    .await
    .into_iter()
    .collect::<Vec<anyhow::Result<Entry>>>();

    Ok(())
}

async fn write_to_file(response: Response, entry: Entry) -> anyhow::Result<Entry> {
    // Turn into bytes
    let bytes = response.text().await?;
    let file_name = format!("{}.{}", &entry.og_title, &entry.format.extension());
    let path = Path::new(OUTPUT).join(&file_name);
    let mut file = File::create(path)?;
    // Write to file
    copy(&mut bytes.as_bytes(), &mut file)?;
    Ok(entry)
}
