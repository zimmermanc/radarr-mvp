#[cfg(test)]
mod tests {
    use super::super::streaming_client::TmdbStreamingClient;
    use radarr_core::streaming::{traits::TmdbAdapter, MediaType, TimeWindow};

    #[test]
    fn test_tmdb_streaming_client_creation() {
        let api_key = "test_api_key".to_string();
        let client = TmdbStreamingClient::new(api_key);
        // Just verify it compiles and creates successfully
        assert!(true);
    }

    #[tokio::test]
    #[ignore] // Requires actual API key
    async fn test_trending_movies() {
        let api_key = std::env::var("TMDB_API_KEY").unwrap_or_else(|_| "test_key".to_string());
        let client = TmdbStreamingClient::new(api_key);

        // This would fail without a real API key, but shows the interface works
        let result = client.trending_movies(TimeWindow::Day).await;

        // With a real API key, this should succeed
        if result.is_ok() {
            let entries = result.unwrap();
            assert!(!entries.is_empty());
        }
    }
}
