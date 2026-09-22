mod adapters;
mod application;
mod bootstrap;
mod domain;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    crate::bootstrap::run().await
}
