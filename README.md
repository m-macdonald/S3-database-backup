# database-backup
[![build](https://github.com/m-macdonald/S3-database-backup/actions/workflows/build.yml/badge.svg)](https://github.com/m-macdonald/S3-database-backup/actions/workflows/build.yml)

Restoring a local DB for testing

`tar -xvf` the file to decompress and unzip it
`pg_restore -d {connection string here} {path to newly unzipped dump file}`

create a .env file in the root of this project with the following structure:
```
DATABASE_URL=
DATABASE_SCHEMA_PATTERN=
AWS_S3_BUCKET=
AWS_S3_ENDPOINT=
AWS_S3_REGION=
```

build the project with
`nix build .#devImage && ./result | docker load`

run it
`docker compose up`

remote into the container with
`docker exec -it {container name} bash`
