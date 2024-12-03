use actix_web::{web, App, HttpServer};
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

mod notifications;
mod services;
mod handlers;
mod utils;

use services::NotificationManager;
use utils::constants::{APP_ID, APP_DISPLAY_NAME};

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("debug"));
    
    log::info!("Initializing notification manager...");
    let manager = Arc::new(Mutex::new(
        NotificationManager::new(APP_ID, APP_DISPLAY_NAME)
            .await
            .context("Failed to create notification manager")?
    ));
    log::info!("Notification manager initialized successfully");

    println!("Starting notification server on http://localhost:3000");
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(manager.clone()))
            .route("/notify", web::post().to(handlers::send_notification))
    })
    .bind("0.0.0.0:3000")?
    .workers(4)
    .run()
    .await?;

    Ok(())
}
