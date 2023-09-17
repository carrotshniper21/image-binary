mod models;
mod utils;

use axum::{http::Uri, response::Response, routing::post, Json, Router};
use http::{Method, StatusCode};
use image::GenericImageView;
use log::info;
use std::{error::Error, path, str::FromStr, time::Duration};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::Span;

async fn convert_to_binary(image_path: &str) -> Result<String, Box<dyn Error>> {
    let image = image::open(image_path)?;

    let (width, height) = image.dimensions();
    let mut image_binary = String::new();

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            image_binary += &format!("{:08b}{:08b}{:08b}", pixel[0], pixel[1], pixel[2]);
        }
    }

    Ok(image_binary)
}

async fn convert_to_hex(image_path: &str) -> Result<String, Box<dyn Error>> {
    let image = image::open(image_path)?;

    let (width, height) = image.dimensions();
    let mut image_hex = String::new();

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            image_hex += &format!("{:02X}{:02X}{:02X}", pixel[0], pixel[1], pixel[2]);
        }
    }

    Ok(image_hex)
}

async fn fallback_func() -> (StatusCode, Json<models::ResponseError>) {
    (
        StatusCode::NOT_FOUND,
        Json(models::ResponseError {
            message: String::new(),
            error: String::from("page not found"),
        }),
    )
}

async fn image_upload(
    Json(payload): Json<models::FileUpload>,
) -> (StatusCode, Json<models::FileResponse>) {
    if !path::Path::new("./temp").exists() {
        std::fs::create_dir("temp").expect("Error: Failed to create temp directory");
    }

    let image_data = openssl::base64::decode_block(&payload.file.contents)
        .expect("Error: Failed to decode file contents");

    std::fs::write("temp/temp.jpg", image_data).expect("Error: Failed to write temp file");
    let binary_data = convert_to_binary("./temp/temp.jpg")
        .await
        .expect("Error: Failed to convert image to binary");

    let hex_data = convert_to_hex("./temp/temp.jpg")
        .await
        .expect("Error: Failed to convert image to hexadecimal");

    std::fs::write("./temp/hex-cache.txt", &hex_data).unwrap();

    return (
        StatusCode::OK,
        Json(models::FileResponse {
            binary: binary_data,
            hex: hex_data,
        }),
    );
}

struct RequestUri(Uri);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    utils::logger::initialize();

    let cors = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_headers(Any)
        .allow_origin(Any);

    let app = Router::new()
        .route("/upload", post(image_upload))
        .fallback(fallback_func)
        .layer(cors)
        .layer(TraceLayer::new_for_http().on_response(
            |response: &Response, _latency: Duration, _span: &Span| {
                println!(
                    "{:?}",
                    response.extensions().get::<RequestUri>().map(|r| &r.0)
                )
            },
        ));

    let addr = std::net::SocketAddr::from_str(&format!("0.0.0.0:8080")).unwrap();
    info!("Server started on http://{}\n", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
