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
use utils::auth::{AuthConfig, AuthMiddleware};

#[derive(Parser, Debug)]
#[command(author, version, about = "Notification server for sending Windows notifications")]
struct Args {
    /// Address to listen on
    #[arg(short, long, default_value = "0.0.0.0")]
    address: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Optional username for basic authentication
    #[arg(short, long)]
    username: Option<String>,

    /// Optional password for basic authentication
    #[arg(short = 'w', long)]
    password: Option<String>,
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
    let is_localhost = args.address == "127.0.0.1" || args.address == "localhost" || args.address == "::1";
    
    // Initialize auth config
    let auth_config = AuthConfig::new(args.username, args.password);
    if auth_config.is_auth_required() {
        println!("Basic authentication enabled");
    }
    
    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(manager.clone()))
            .wrap(AuthMiddleware::new(auth_config.clone()))
            .route("/notify", web::post().to(handlers::send_notification))
    })
    .bind(&bind_addr)?;

    // Only bind to localhost if we're not already bound to it
    if !is_localhost {
        let localhost_addr = format!("127.0.0.1:{}", args.port);
        server = server.bind(&localhost_addr)?;
        println!("Starting notification server on http://{} and http://{}", bind_addr, localhost_addr);
    } else {
        println!("Starting notification server on http://{}", bind_addr);
    }
    
    server.workers(4).run().await?;

    Ok(())
}
