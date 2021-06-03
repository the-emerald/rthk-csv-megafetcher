use crate::schema::Format::{Audio, Video};
use crate::schema::Language::{Chinese, English};
use crate::schema::{Entry, Format, Language};
use anyhow::anyhow;
use clap::{load_yaml, App};
use futures::TryFutureExt;
use std::fs::{File, create_dir_all};
use std::io::{copy, BufReader};
use std::path::Path;
use std::str::FromStr;

pub mod schema;

pub const OUTPUT: &str = "./output";

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

    for line in reader.deserialize() {
        let record: Entry = line?;
        to_fetch.push(record);
    }

    // Filter
    let to_fetch = to_fetch
        .into_iter()
        .filter(|entry| language.contains(&entry.language))
        .filter(|entry| format.contains(&entry.format))
        .collect::<Vec<_>>();

    // Set up what we need for fetching
    let semaphore = tokio::sync::Semaphore::new(12);
    let client = reqwest::Client::new();
    create_dir_all(Path::new(OUTPUT))?;

    let fetched = futures::future::join_all(to_fetch.into_iter().map(|entry| async {
        // Get semaphore
        let _permit = semaphore.acquire().await?;

        // Reqwest from client
        client
            .get(entry.file_url.clone())
            .send()
            .map_err(|e| anyhow!(e))
            .and_then(|response| async {
                // Turn into bytes
                let bytes = response.text().await?;
                let path = Path::new(OUTPUT).join(&entry.og_title);
                let mut file = File::create(path)?;

                // Write to file
                copy(&mut bytes.as_bytes(), &mut file)?;

                // Keep the entry
                Ok(entry)
            })
            .await
    }))
    .await
    .into_iter()
    .collect::<Vec<anyhow::Result<Entry>>>();

    Ok(())
}
