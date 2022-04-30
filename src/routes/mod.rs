use poem_openapi::ApiResponse;

pub mod bots;
pub mod packs;

#[derive(Debug, ApiResponse)]
pub enum StandardResponse {
    /// The operation was successful
    #[oai(status = 200)]
    Ok,

    /// The id is invalid.
    #[oai(status = 400)]
    BadRequest,
}