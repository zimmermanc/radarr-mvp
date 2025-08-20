//! Movie API routes

use crate::handlers::movies;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

/// Create movie-related routes
pub fn create_movie_routes() -> Router {
    Router::new()
        .route("/movie", get(movies::list_movies))
        .route("/movie", post(movies::create_movie))
        .route("/movie/:id", get(movies::get_movie))
        .route("/movie/:id", put(movies::update_movie))
        .route("/movie/:id", delete(movies::delete_movie))
}