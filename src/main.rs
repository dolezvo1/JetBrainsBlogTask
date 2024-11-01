use axum::{
    extract::Extension,
    routing::get,
    Router,
};
use clap::Parser;
use sqlx::SqlitePool;
use std::{fs::File, sync::Arc};

mod db;
mod endpoints;

const FRONTPAGE_LOCATION: &str = "/home";
const DATA_LOCATION: &str = "/data";

#[derive(Parser)]
struct CliOptions {
    #[arg(
        long,
        help = "Specify the database file path (optional, runs in memory when not set)"
    )]
    db_file: Option<String>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli_options = CliOptions::parse();

    let pool = match cli_options.db_file {
        // Store db to a file
        Some(file_name) => {
            let first_usage = File::open(&file_name).is_err();

            if first_usage {
                File::create(&file_name)?;
            }

            let pool = Arc::new(
                SqlitePool::connect(&format!("sqlite://{}", file_name))
                    .await
                    .unwrap(),
            );

            if first_usage {
                db::setup_database(&pool).await;
            }

            pool
        }
        // Use in-memory db
        None => {
            let pool = Arc::new(SqlitePool::connect("sqlite::memory:").await.unwrap());
            db::setup_database(&pool).await;
            pool
        }
    };

    let app = Router::new()
        .route(
            FRONTPAGE_LOCATION,
            get(endpoints::frontpage).post(endpoints::add_post),
        )
        .route(
            &format!("{}/:file_id", DATA_LOCATION),
            get(endpoints::serve_data),
        )
        .layer(Extension(pool));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
