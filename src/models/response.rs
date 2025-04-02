use serde::Serialize;
use rocket::serde::json::Json;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: impl Into<String>) -> Json<Self> {
        Json(Self {
            success: true,
            message: message.into(),
            data: Some(data)
        })
    }

    pub fn error(message: impl Into<String>) -> Json<Self> {
        Json(Self {
            success: false,
            message: message.into(),
            data: None
        })
    }
}

// Helper function to convert any error into an API error response
pub fn into_api_error<E: std::fmt::Display>(error: E) -> Json<ApiResponse<()>> {
    ApiResponse::error(error.to_string())
} 