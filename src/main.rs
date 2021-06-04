use crate::schema::Format::{Audio, Video};
use crate::schema::Language::{Chinese, English};
use crate::schema::{Entry, Format, Id, Language};
use anyhow::anyhow;
use clap::{load_yaml, App};
use core::time::Duration;
use futures::TryFutureExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Response;
use std::collections::HashSet;
use std::fs::{create_dir_all, File};
use std::io::{copy, BufReader, BufWriter};
use std::path::Path;
use std::str::FromStr;

pub mod schema;

pub const DOWNLOADED_JSON: &str = "downloaded.json";

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
    let output_dir = Path::new(args.value_of("output").unwrap());
    let force = args.is_present("force");

    create_dir_all(output_dir)?;

    // Read list of already-downloaded files
    let mut downloaded_files: HashSet<Id> = if output_dir.join(DOWNLOADED_JSON).exists() {
        serde_json::from_reader(BufReader::new(File::open(
            output_dir.join(DOWNLOADED_JSON),
        )?))?
    } else {
        File::create(output_dir.join(DOWNLOADED_JSON))?;
        HashSet::new()
    };

    // Read CSV
    let mut to_fetch = Vec::new();
    let mut reader = csv::Reader::from_reader(source);
    let csv_reading_pbar = {
        let pb = ProgressBar::new_spinner().with_style(
            ProgressStyle::default_spinner().template("{prefix:.bold.dim} {spinner} {wide_msg}"),
        );
        pb.set_message("Reading manifest");
        pb.set_prefix("[1/2]");
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

    // If not force, then also filter files already downloaded
    let to_fetch = if !force {
        to_fetch
            .into_iter()
            .filter(|entry| !downloaded_files.contains(&entry.id))
            .collect()
    } else {
        to_fetch
    };

    // Set up what we need for fetching
    let semaphore = tokio::sync::Semaphore::new(16);
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(15))
        .build()?;

    let currently_downloading_pbar = MultiProgress::new();
    let total_progress_pb = currently_downloading_pbar.add(ProgressBar::new(to_fetch.len() as u64).with_style(
        ProgressStyle::default_bar()
            .template("{prefix:.bold.dim} {spinner} {msg} [{elapsed_precise}] [{wide_bar}] {pos}/{len} ({eta_precise})")
    )
        .with_message("Fetching content")
        .with_prefix("[2/2]"));

    let (ok, failed): (Vec<_>, Vec<_>) =
        futures::future::join_all(to_fetch.into_iter().map(|entry| async {
            // Get semaphore
            let _permit = semaphore.acquire().await?;

            // Set up pbar
            let name_bar = currently_downloading_pbar.add(
                ProgressBar::new_spinner()
                    .with_style(
                        ProgressStyle::default_spinner().template("{prefix:.bold.dim} {wide_msg}"),
                    )
                    .with_message(entry.og_title.clone())
                    .with_prefix("[DL]"),
            );
            name_bar.inc(1);

            // Make request from client
            let r = client
                .get(entry.file_url.clone())
                .send()
                .map_err(|e| anyhow!(e))
                .and_then(|response| write_to_file(response, entry, output_dir))
                .await;

            total_progress_pb.inc(1);
            name_bar.finish_and_clear();
            r
        }))
        .await
        .into_iter()
        .collect::<Vec<anyhow::Result<Entry>>>()
        .into_iter()
        .partition(Result::is_ok);

    // Update downloaded set
    for x in &ok {
        // Always fine since we partitioned already.
        let r = x.as_ref().unwrap();
        downloaded_files.insert(r.id);
    }

    // Write successes to downloaded.json
    serde_json::to_writer(
        BufWriter::new(File::open(output_dir.join(DOWNLOADED_JSON))?),
        &downloaded_files,
    )?;

    println!("Fetched {} files with {} failures.", ok.len(), failed.len());
    if !failed.is_empty() {
        println!("Run the program again to retry failed downloads.");
    }

    Ok(())
}

async fn write_to_file(
    response: Response,
    entry: Entry,
    output_dir: &Path,
) -> anyhow::Result<Entry> {
    let file_name = format!("{}.{}", &entry.episode_title, &entry.format.extension());
    let path = output_dir
        .join(&entry.language.to_string())
        .join(&entry.programme_title.to_string());
    create_dir_all(path.clone())?;
    let path = path.join(&file_name);
    let mut file = File::create(path)?;

    // Turn into bytes and write to file
    let bytes = response.text().await?;
    copy(&mut bytes.as_bytes(), &mut file)?;
    Ok(entry)
}
