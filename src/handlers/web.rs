use std::sync::{Arc, Mutex};
use actix_web::{web, HttpResponse, Error, HttpRequest};
use std::time::Instant;
use actix_multipart::Multipart;
use futures_util::TryStreamExt;
use std::io::Write;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;
use std::env;
use serde::Deserialize;
use bytes::BytesMut;
use futures_util::StreamExt;

use crate::services::NotificationManager;
use crate::notifications::NotificationRequest;

const NOTIFICATION_ASSETS_DIR: &str = "notification_server_assets";

#[derive(Deserialize)]
struct FormData {
    title: Option<String>,
    message: Option<String>,
}

async fn handle_multipart(
    mut payload: Multipart,
    temp_dir: PathBuf,
) -> Result<NotificationRequest, Error> {
    let mut title = String::new();
    let mut message = String::new();
    let mut image_path = None;
    let mut file_paths = Vec::new();
    let mut callback_command = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let name = content_disposition.get_name().unwrap_or("");

        match name {
            "title" => {
                let mut content = Vec::new();
                while let Ok(Some(chunk)) = field.try_next().await {
                    content.extend_from_slice(&chunk);
                }
                title = String::from_utf8(content)
                    .map_err(|e| {
                        log::error!("Invalid UTF-8 in title: {}", e);
                        actix_web::error::ErrorBadRequest("Invalid title encoding")
                    })?;
            },
            "message" => {
                let mut content = Vec::new();
                while let Ok(Some(chunk)) = field.try_next().await {
                    content.extend_from_slice(&chunk);
                }
                message = String::from_utf8(content)
                    .map_err(|e| {
                        log::error!("Invalid UTF-8 in message: {}", e);
                        actix_web::error::ErrorBadRequest("Invalid message encoding")
                    })?;
            },
            "callback_command" => {
                let mut content = Vec::new();
                while let Ok(Some(chunk)) = field.try_next().await {
                    content.extend_from_slice(&chunk);
                }
                let cmd = String::from_utf8(content)
                    .map_err(|e| {
                        log::error!("Invalid UTF-8 in callback_command: {}", e);
                        actix_web::error::ErrorBadRequest("Invalid callback_command encoding")
                    })?;
                callback_command = Some(cmd);
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
                    
                    let mut file = fs::File::create(&file_path)
                        .map_err(|e| {
                            log::error!("Failed to create image file: {}", e);
                            actix_web::error::ErrorInternalServerError("Failed to create image file")
                        })?;

                    while let Ok(Some(chunk)) = field.try_next().await {
                        file.write_all(&chunk)
                            .map_err(|e| {
                                log::error!("Failed to write image chunk: {}", e);
                                actix_web::error::ErrorInternalServerError("Failed to save image")
                            })?;
                    }
                    image_path = Some(file_path.to_string_lossy().into_owned());
                }
            },
            "files" => {
                if let Some(filename) = content_disposition.get_filename() {
                    let file_path = temp_dir.join(filename);
                    
                    let mut file = fs::File::create(&file_path)
                        .map_err(|e| {
                            log::error!("Failed to create file: {}", e);
                            actix_web::error::ErrorInternalServerError("Failed to create file")
                        })?;

                    while let Ok(Some(chunk)) = field.try_next().await {
                        file.write_all(&chunk)
                            .map_err(|e| {
                                log::error!("Failed to write file chunk: {}", e);
                                actix_web::error::ErrorInternalServerError("Failed to save file")
                            })?;
                    }
                    file_paths.push(file_path.to_string_lossy().into_owned());
                }
            },
            _ => {
                log::warn!("Unexpected field: {}", name);
            }
        }
    }

    Ok(NotificationRequest {
        title,
        message,
        notification_type: Default::default(),
        image_path,
        file_paths: if file_paths.is_empty() { None } else { Some(file_paths) },
        callback_command,
    })
}

pub async fn send_notification(
    req: HttpRequest,
    mut payload: web::Payload,
    manager: web::Data<Arc<Mutex<NotificationManager>>>,
) -> Result<HttpResponse, Error> {
    let start = Instant::now();
    log::info!("Received notification request at {:?}", start);
    
    // Create temporary directory for this notification
    let notification_id = Uuid::new_v4();
    let temp_base = env::temp_dir().join(NOTIFICATION_ASSETS_DIR);
    let temp_dir = temp_base.join(notification_id.to_string());
    fs::create_dir_all(&temp_dir)
        .map_err(|e| {
            log::error!("Failed to create temp directory: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to create temp directory")
        })?;

    // Get content type from request headers
    let content_type = req.headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("");

    // Handle request based on content type
    let request = if content_type.starts_with("multipart/form-data") {
        handle_multipart(Multipart::new(req.headers(), payload), temp_dir).await?
    } else {
        // Handle URL-encoded form data
        let mut body = BytesMut::new();
        while let Some(chunk) = payload.next().await {
            let chunk = chunk?;
            body.extend_from_slice(&chunk);
        }
        
        let form_data: FormData = serde_urlencoded::from_bytes(&body)
            .map_err(|_| actix_web::error::ErrorBadRequest("Invalid form data"))?;

        NotificationRequest {
            title: form_data.title.unwrap_or_default(),
            message: form_data.message.unwrap_or_default(),
            notification_type: Default::default(),
            image_path: None,
            file_paths: None,
            callback_command: None,
        }
    };

    // Send notification
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
