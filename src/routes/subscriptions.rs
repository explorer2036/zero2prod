use actix_web::{web, HttpResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// Extract form data using serde.
pub async fn subscribe(form: web::Form<FormData>) -> HttpResponse {
    println!("{}, {}", form.name, form.email);
    HttpResponse::Ok().finish()
}
