use std::sync::{Arc, Mutex};
use actix_web::{web, HttpResponse, Responder};
use std::time::Instant;

use crate::services::NotificationManager;
use crate::notifications::NotificationRequest;

pub async fn send_notification(
    manager: web::Data<Arc<Mutex<NotificationManager>>>,
    request: web::Json<NotificationRequest>,
) -> impl Responder {
    let start = Instant::now();
    log::info!("Received notification request at {:?}", start);
    
    let mut manager = manager.lock().unwrap();
    let result = match manager.send_notification(request.0).await {
        Ok(_) => {
            log::info!("Request completed successfully in {:?}", start.elapsed());
            HttpResponse::Ok().body("Notification sent successfully")
        },
        Err(e) => {
            log::error!("Failed to send notification: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to send notification: {}", e))
        }
    };
    
    log::info!("Total request handling time: {:?}", start.elapsed());
    result
}
