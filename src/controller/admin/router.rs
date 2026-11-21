use crate::state::SharedState;

use super::auth_handler::*;
use super::cat_handler::*;
use super::image_handler::*;
use super::index_handler::*;
use super::order_handler::*;

pub fn router() -> axum::Router<SharedState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout))
        .route("/", get(dashboard))
        .route("/categories", get(categories_page).post(category_create))
        .route("/categories/:id", post(category_update))
        .route("/categories/:id/delete", post(category_delete))
        .route("/images", get(images_page))
        .route("/images/new", get(image_new_page).post(image_create))
        .route("/images/:id/edit", get(image_edit_page).post(image_update))
        .route("/images/:id/delete", post(image_delete))
        .route("/images/:id/toggle", post(image_toggle))
        .route("/orders", get(orders_page))
}
