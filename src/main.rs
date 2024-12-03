use actix_web::{web, App, HttpServer};
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use clap::Parser;

mod notifications;
mod services;
mod handlers;
mod utils;

use services::NotificationManager;
use utils::constants::{APP_ID, APP_DISPLAY_NAME};

#[derive(Parser, Debug)]
#[command(author, version, about = "Notification server for sending Windows notifications")]
struct Args {
    /// Address to listen on
    #[arg(short, long, default_value = "0.0.0.0")]
    address: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

#[actix_web::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("debug"));
    
    log::info!("Initializing notification manager...");
    let manager = Arc::new(Mutex::new(
        NotificationManager::new(APP_ID, APP_DISPLAY_NAME)
            .await
            .context("Failed to create notification manager")?
    ));
    log::info!("Notification manager initialized successfully");

    let bind_addr = format!("{}:{}", args.address, args.port);
    println!("Starting notification server on http://{}", bind_addr);
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(manager.clone()))
            .route("/notify", web::post().to(handlers::send_notification))
    })
    .bind(&bind_addr)?
    .workers(4)
    .run()
    .await?;

    Ok(())
}
