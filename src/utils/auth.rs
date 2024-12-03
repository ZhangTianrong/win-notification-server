use actix_web::{
    dev::{ServiceRequest, Service, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    http::header,
    Error,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::future::{ready, Ready, Future};
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl AuthConfig {
    pub fn new(username: Option<String>, password: Option<String>) -> Self {
        Self { username, password }
    }

    pub fn is_auth_required(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }

    pub fn validate_auth_header(&self, req: &ServiceRequest) -> Result<bool, Error> {
        // Allow requests from localhost without authentication
        if is_localhost(req) {
            return Ok(true);
        }

        // If no auth is configured, allow all requests
        if !self.is_auth_required() {
            return Ok(true);
        }

        let auth_header = match req.headers().get(header::AUTHORIZATION) {
            Some(header) => header,
            None => return Err(ErrorUnauthorized("Missing authorization header")),
        };

        let auth_str = match auth_header.to_str() {
            Ok(str) => str,
            Err(_) => return Err(ErrorUnauthorized("Invalid authorization header")),
        };

        if !auth_str.starts_with("Basic ") {
            return Err(ErrorUnauthorized("Invalid authorization type"));
        }

        let credentials = match STANDARD.decode(&auth_str[6..]) {
            Ok(decoded) => match String::from_utf8(decoded) {
                Ok(str) => str,
                Err(_) => return Err(ErrorUnauthorized("Invalid authorization header")),
            },
            Err(_) => return Err(ErrorUnauthorized("Invalid authorization header")),
        };

        let parts: Vec<&str> = credentials.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ErrorUnauthorized("Invalid credentials format"));
        }

        if parts[0] == self.username.as_ref().unwrap() && parts[1] == self.password.as_ref().unwrap() {
            Ok(true)
        } else {
            Err(ErrorUnauthorized("Invalid credentials"))
        }
    }
}

fn is_localhost(req: &ServiceRequest) -> bool {
    if let Some(addr) = req.connection_info().realip_remote_addr() {
        return addr.starts_with("127.0.0.1") || addr.starts_with("::1") || addr.starts_with("localhost");
    }
    false
}

pub struct AuthMiddleware {
    auth_config: AuthConfig,
}

impl AuthMiddleware {
    pub fn new(auth_config: AuthConfig) -> Self {
        Self { auth_config }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service,
            auth_config: self.auth_config.clone(),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
    auth_config: AuthConfig,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_result = self.auth_config.validate_auth_header(&req);
        let fut = self.service.call(req);

        Box::pin(async move {
            auth_result?;
            fut.await
        })
    }
}
