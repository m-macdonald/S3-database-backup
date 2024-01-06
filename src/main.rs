use std::{
    process::{Command, exit},
    io, fs, path::Path
};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{config::Region, primitives::ByteStream, Client};
use url::Url;
use envconfig::Envconfig;
use chrono::prelude::Utc;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let config = Config::init_from_env().unwrap();

    let url = &config.database_url;
    let url = Url::parse(url)
        .expect("Failed to parse the provided database url");
    let db_type = url.scheme();
    let db_name = url.path_segments()
        .expect("The given url is missing the database name segment")
        .last()
        .expect("The database name could not be determined from the given url");
    let db_hostname = url.host();
    let db_user = url.username();
    let db_password = url.password();
    let db_port = url.port();

    let timestamp = Utc::now().format("%Y-%m-%d_%H:%M:%S");

    let filename = format!("backup-{db_name}-{timestamp}.tar.gz");
    let filepath = format!("/tmp/{filename}");
    let filepath_dump = format!("{filepath}.dump");

    println!("Backup beginning on {db_name}...");
    let mut command_args = vec!(
            "-d",
            url.as_str(),
            "-F",
            "c",
            "-f",
            &filepath_dump,
        );

    match &config.database_schema_pattern {
        Some(database_schema_pattern) => {
            println!("The following schema pattern was provided and will be used: {database_schema_pattern}");
            command_args.push("-n");
            command_args.push(database_schema_pattern)
        },
        None => println!("No database schema pattern provided. All schemas will be dumped.")
    }

    let output = Command::new("pg_dump")
        .args(command_args)
        .output()
        .expect("pg_dump failed");
    
    match String::from_utf8(output.stdout) {
        Ok(err) => println!("{err}"),
        Err(err) => println!("Failed to process stdout from pg_dump: {err}")
    }

    if output.stderr.last().is_some() {
        match String::from_utf8(output.stderr) {
            Ok(err) => panic!("{err}"),
            Err(err) => panic!("Failed to process stderr from pg_dump: {err}")
        }
    }

    println!("Database dump successful");

    println!("Beginning zip of dump...");
    let output = Command::new("tar")
        .args([
            "-czvf",
            &filepath,
            &filepath_dump
        ])
        .output();
    // TODO : Need to handle stderr as done above for pg_dump since many errors don't seem to be
    // translating to Rust Results
    // TODO : I'm sure there's a better way of handling this error
    output.map_err(|err| {
        println!("Zip failed with the following error {err}");
        println!("Cleaning files and exiting");
        fs::remove_file(filepath_dump)
            .expect("Failed to delete database dump file");
        println!("Dump file successfully deleted");

        exit(1);
    }).unwrap();

    let file_stream = ByteStream::from_path(Path::new(&filepath))
        .await
        .expect("Failed to read file bytestream");

    let s3_region = Region::new(config.aws_s3_region);

    let region_provider = RegionProviderChain::first_try(s3_region)
        .or_default_provider();

    let shared_config = aws_config::from_env()
        .region(region_provider)
        .load()
        .await;

    let s3_client = Client::new(&shared_config);

    let s3_result = s3_client
        .put_object()
        .bucket(&config.aws_s3_bucket)
        .key(filename)
        .body(file_stream)
        .send()
        .await;

    match s3_result {
        Ok(_) => println!("Upload to S3 successful"),
        Err(_) => println!("Upload to S3 failed")
    };

    Ok(())
}

#[derive(Envconfig)]
struct Config {
    #[envconfig(from = "DATABASE_URL")]
    database_url: String,

    #[envconfig(from = "DATABASE_SCHEMA_PATTERN")]
    database_schema_pattern: Option<String>,

    #[envconfig(from = "AWS_S3_BUCKET")]
    aws_s3_bucket: String,
    
    #[envconfig(from = "AWS_S3_ENDPOINT")]
    aws_s3_endpoint: String,

    #[envconfig(from = "AWS_S3_REGION")]
    aws_s3_region: String
}
