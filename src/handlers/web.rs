use std::sync::{Arc, Mutex};
use actix_web::{web, HttpResponse, Error};
use std::time::Instant;
use actix_multipart::Multipart;
use futures_util::TryStreamExt;
use std::io::Write;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;
use std::env;

use crate::services::NotificationManager;
use crate::notifications::NotificationRequest;

const NOTIFICATION_ASSETS_DIR: &str = "notification_server_assets";

pub async fn send_notification(
    manager: web::Data<Arc<Mutex<NotificationManager>>>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let start = Instant::now();
    log::info!("Received notification request at {:?}", start);
    
    let mut title = String::new();
    let mut message = String::new();
    let mut image_path = None;
    let mut file_paths = Vec::new();

    // Create temporary directory for this notification in system temp dir
    let notification_id = Uuid::new_v4();
    let temp_base = env::temp_dir().join(NOTIFICATION_ASSETS_DIR);
    let temp_dir = temp_base.join(notification_id.to_string());
    match fs::create_dir_all(&temp_dir) {
        Ok(_) => (),
        Err(e) => {
            log::error!("Failed to create temp directory: {}", e);
            return Ok(HttpResponse::InternalServerError().body(format!("Failed to create temp directory: {}", e)));
        }
    }

    // Process multipart form data
    while let Some(mut field) = match payload.try_next().await {
        Ok(Some(field)) => Some(field),
        Ok(None) => None,
        Err(e) => {
            log::error!("Error processing multipart data: {}", e);
            return Ok(HttpResponse::BadRequest().body("Invalid form data"));
        }
    } {
        let content_disposition = field.content_disposition().clone();
        let name = content_disposition.get_name().unwrap_or("");

        match name {
            "title" => {
                let mut content = Vec::new();
                while let Some(chunk) = match field.try_next().await {
                    Ok(Some(chunk)) => Some(chunk),
                    Ok(None) => None,
                    Err(e) => {
                        log::error!("Error reading title field: {}", e);
                        return Ok(HttpResponse::BadRequest().body("Invalid title data"));
                    }
                } {
                    content.extend_from_slice(&chunk);
                }
                title = match String::from_utf8(content) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Invalid UTF-8 in title: {}", e);
                        return Ok(HttpResponse::BadRequest().body("Invalid title encoding"));
                    }
                };
            },
            "message" => {
                let mut content = Vec::new();
                while let Some(chunk) = match field.try_next().await {
                    Ok(Some(chunk)) => Some(chunk),
                    Ok(None) => None,
                    Err(e) => {
                        log::error!("Error reading message field: {}", e);
                        return Ok(HttpResponse::BadRequest().body("Invalid message data"));
                    }
                } {
                    content.extend_from_slice(&chunk);
                }
                message = match String::from_utf8(content) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Invalid UTF-8 in message: {}", e);
                        return Ok(HttpResponse::BadRequest().body("Invalid message encoding"));
                    }
                };
            },
            "image" => {
                if let Some(filename) = content_disposition.get_filename() {
                    let input_path = PathBuf::from(filename);
                    let file_ext = input_path.extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("jpg")
                        .to_string();
                    
                    let image_filename = format!("image.{}", file_ext);
                    let file_path = temp_dir.join(&image_filename);
                    
                    let mut file = match fs::File::create(&file_path) {
                        Ok(file) => file,
                        Err(e) => {
                            log::error!("Failed to create image file: {}", e);
                            return Ok(HttpResponse::InternalServerError().body("Failed to create image file"));
                        }
                    };

                    while let Some(chunk) = match field.try_next().await {
                        Ok(Some(chunk)) => Some(chunk),
                        Ok(None) => None,
                        Err(e) => {
                            log::error!("Error reading image data: {}", e);
                            return Ok(HttpResponse::BadRequest().body("Invalid image data"));
                        }
                    } {
                        if let Err(e) = file.write_all(&chunk) {
                            log::error!("Failed to write image chunk: {}", e);
                            return Ok(HttpResponse::InternalServerError().body("Failed to save image"));
                        }
                    }
                    image_path = Some(file_path.to_string_lossy().into_owned());
                }
            },
            "files" => {
                if let Some(filename) = content_disposition.get_filename() {
                    let file_path = temp_dir.join(filename);
                    
                    let mut file = match fs::File::create(&file_path) {
                        Ok(file) => file,
                        Err(e) => {
                            log::error!("Failed to create file: {}", e);
                            return Ok(HttpResponse::InternalServerError().body("Failed to create file"));
                        }
                    };

                    while let Some(chunk) = match field.try_next().await {
                        Ok(Some(chunk)) => Some(chunk),
                        Ok(None) => None,
                        Err(e) => {
                            log::error!("Error reading file data: {}", e);
                            return Ok(HttpResponse::BadRequest().body("Invalid file data"));
                        }
                    } {
                        if let Err(e) = file.write_all(&chunk) {
                            log::error!("Failed to write file chunk: {}", e);
                            return Ok(HttpResponse::InternalServerError().body("Failed to save file"));
                        }
                    }
                    file_paths.push(file_path.to_string_lossy().into_owned());
                }
            },
            _ => {
                log::warn!("Unexpected field: {}", name);
            }
        }
    }

    let request = NotificationRequest {
        title,
        message,
        image_path,
        file_paths: if file_paths.is_empty() { None } else { Some(file_paths) },
        xml_payload: None,
        callback_command: None,
    };

    let mut manager = manager.lock().unwrap();
    match manager.send_notification(request).await {
        Ok(_) => {
            log::info!("Request completed successfully in {:?}", start.elapsed());
            Ok(HttpResponse::Ok().body("Notification sent successfully"))
        },
        Err(e) => {
            log::error!("Failed to send notification: {}", e);
            Ok(HttpResponse::InternalServerError().body(format!("Failed to send notification: {}", e)))
        }
    }
}
