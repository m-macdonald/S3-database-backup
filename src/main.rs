use std::{
    process::{Command, exit},
    io, fs, path::Path
};
use aws_config::meta::{region::RegionProviderChain};
use aws_sdk_s3::{config::Region, primitives::ByteStream, Client};
use dotenvy;
use url::Url;
use envconfig::Envconfig;
use chrono::prelude::Utc;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    dotenvy::dotenv();
    let config = Config::init_from_env().unwrap();
/*
    let env_keys = ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_S3_REGION", "AWS_S3_ENDPOINT", "AWS_S3_BUCKET", "DATABASE_URL"];


    let (env_vars, errors): (HashMap<&str, String>, HashMap<&str, dotenvy::Error>) = env_keys
        .into_iter()
        .partition_map(|env_key| { 
            match dotenvy::var(env_key) {
                Ok(env_var) => Either::Left((env_key, env_var)),
                Err(err) => Either::Right((env_key, err))
            }
        });

    if !errors.is_empty() {
        panic!("The following environment variables could not be processed successfully:\n{:}", errors.into_keys().into_iter().format("\n"))
    }
    */
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

    Command::new("pg_dump")
        .args([
            url.as_str(),
            "-F",
            "c",
            ">",
            &filepath_dump
        ])
        .output()
        .expect("pg_dump failed");

    println!("Database dump successful");

    println!("Beginning zip of dump...");
    let output = Command::new("tar")
        .args([
            "-czvf",
            &filepath,
            &filepath_dump
        ])
        .output();

    output.map_err(|err| {
        println!("Zip failed with the following error {err}");
        println!("Cleaning files and exiting");
        fs::remove_file(filepath_dump)
            .expect("Failed to delete database dump file");
        println!("Dump file successfully deleted");

        exit(1);
    }).unwrap();

    // TODO : Read in zip file
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

    #[envconfig(from = "AWS_S3_BUCKET")]
    aws_s3_bucket: String,
    
    #[envconfig(from = "AWS_S3_ENDPOINT")]
    aws_s3_endpoint: String,

    #[envconfig(from = "AWS_S3_REGION")]
    aws_s3_region: String

}


/*
enum EnvVars {
    AWS_ACCESS_KEY = "AWS_ACCESS_KEY_ID", 
    AWS_SECRET_ACCESS_KEY = "AWS_SECRET_ACCESS_KEY",
    AWS_S3_REGION = "AWS_S3_REGION",
    AWS_S3_ENDPOINT = "AWS_S3_ENDPOINT",
    AWS_S3_BUCKET = "AWS_S3_BUCKET",
    DATABASE_URL = "DATABASE_URL",
}

impl EnvVars {
    
}
    */
