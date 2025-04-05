use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: String) -> Json<Self> {
        Json(ApiResponse {
            success: true,
            message,
            data: Some(data),
        })
    }

    pub fn error(message: String) -> Json<Self> {
        Json(ApiResponse {
            success: false,
            message,
            data: None,
        })
    }
}

// Helper function to convert any error into an API error response
pub fn into_api_error<E: std::fmt::Display>(error: E) -> Json<ApiResponse<()>> {
    ApiResponse::error(error.to_string())
} 