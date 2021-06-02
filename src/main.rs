use clap::{App, load_yaml};
use std::fs::File;
use crate::schema::{Language, Format, Entry};
use std::str::FromStr;
use crate::schema::Language::{English, Chinese};
use crate::schema::Format::{Audio, Video};
use std::io::BufReader;

pub mod schema;

fn main() -> anyhow::Result<()> {
    let yaml = load_yaml!("cli.yml");
    let args = App::from_yaml(yaml).get_matches();

    // Configs
    let source = BufReader::new(File::open(args.value_of("source").unwrap())?);
    let language = args.value_of("language")
        .map(|lang| Language::from_str(lang))
        .transpose()?
        .map(|lang| vec![lang])
        .unwrap_or(vec![English, Chinese]);
    let format = args.value_of("format")
        .map(|fmt| Format::from_str(fmt))
        .transpose()?
        .map(|fmt| vec![fmt])
        .unwrap_or(vec![Audio, Video]);

    let mut to_fetch = Vec::new();
    let mut reader = csv::Reader::from_reader(source);

    for line in reader.deserialize() {
        let record: Entry = line?;
        to_fetch.push(record);
    }

    Ok(())
}
