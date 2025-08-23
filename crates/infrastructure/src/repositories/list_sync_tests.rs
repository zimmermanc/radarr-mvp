//! Comprehensive tests for List Sync Repository
//!
//! Tests all aspects of sync history management, performance tracking,
//! and audit logging functionality.

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use serde_json::json;
    use sqlx::PgPool;
    use uuid::Uuid;

    /// Helper to create a test database pool
    async fn create_test_pool() -> PgPool {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://test:test@localhost/radarr_test".to_string());
        
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    /// Helper to clean up test data
    async fn cleanup_test_data(pool: &PgPool) {
        sqlx::query!("DELETE FROM list_sync_history WHERE true")
            .execute(pool)
            .await
            .expect("Failed to cleanup test data");
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_sync_lifecycle_basic() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        // Start sync
        let sync_id = repo.start_sync(list_id, None).await.unwrap();
        assert!(!sync_id.is_nil());

        // Complete sync successfully
        repo.complete_sync(
            sync_id,
            "success",
            20,
            10,
            5,
            0,
            5,
            None,
            None,
        ).await.unwrap();

        // Verify in database
        let entry = sqlx::query!(
            "SELECT * FROM list_sync_history WHERE id = $1",
            sync_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(entry.sync_status, "success");
        assert_eq!(entry.items_found, 20);
        assert_eq!(entry.items_added, 10);
        assert_eq!(entry.items_updated, 5);
        assert_eq!(entry.items_excluded, 5);
        assert!(entry.completed_at.is_some());
        assert!(entry.duration_ms.unwrap() > 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_sync_with_error() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        let sync_id = repo.start_sync(list_id, None).await.unwrap();

        // Complete sync with error
        let error_details = json!({
            "error_code": "API_RATE_LIMIT",
            "retry_after": 300,
            "details": "Rate limit exceeded for TMDb API"
        });

        repo.complete_sync(
            sync_id,
            "failed",
            0,
            0,
            0,
            0,
            0,
            Some("Rate limit exceeded".to_string()),
            Some(error_details.clone()),
        ).await.unwrap();

        // Verify error details stored
        let entry = sqlx::query!(
            "SELECT error_message, error_details FROM list_sync_history WHERE id = $1",
            sync_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(entry.error_message.unwrap(), "Rate limit exceeded");
        assert_eq!(entry.error_details.unwrap(), error_details);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_performance_metrics_recording() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        // Start and complete a sync
        let sync_id = repo.start_sync(list_id, None).await.unwrap();
        repo.complete_sync(sync_id, "success", 100, 50, 25, 0, 25, None, None).await.unwrap();

        // Record performance metrics
        let metrics = SyncPerformanceMetrics {
            list_id,
            duration_ms: 5000,
            items_per_second: 20.0,
            memory_peak_mb: Some(256.7),
            network_requests: 25,
            cache_hit_rate: Some(0.85),
            error_rate: 0.02,
            timestamp: Utc::now(),
        };

        repo.record_performance_metrics(&metrics).await.unwrap();

        // Verify metrics stored in sync_metadata
        let entry = sqlx::query!(
            "SELECT sync_metadata FROM list_sync_history WHERE id = $1",
            sync_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let metadata = entry.sync_metadata.unwrap();
        let perf_metrics = &metadata["performance_metrics"];
        
        assert_eq!(perf_metrics["items_per_second"].as_f64().unwrap(), 20.0);
        assert_eq!(perf_metrics["memory_peak_mb"].as_f64().unwrap(), 256.7);
        assert_eq!(perf_metrics["cache_hit_rate"].as_f64().unwrap(), 0.85);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_conflict_resolution_recording() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        let sync_id = repo.start_sync(list_id, None).await.unwrap();

        // Create conflict resolution details
        let conflicts = vec![
            ConflictResolutionDetail {
                movie_tmdb_id: Some(12345),
                movie_imdb_id: Some("tt1234567".to_string()),
                movie_title: "Test Movie".to_string(),
                conflict_type: "duplicate_title".to_string(),
                resolution_strategy: "merge".to_string(),
                existing_data: json!({"title": "Test Movie", "year": 2020}),
                new_data: json!({"title": "Test Movie", "year": 2020, "overview": "New overview"}),
                final_data: json!({"title": "Test Movie", "year": 2020, "overview": "New overview"}),
                resolved_at: Utc::now(),
            },
            ConflictResolutionDetail {
                movie_tmdb_id: Some(67890),
                movie_imdb_id: None,
                movie_title: "Another Movie".to_string(),
                conflict_type: "different_metadata".to_string(),
                resolution_strategy: "keep_existing".to_string(),
                existing_data: json!({"rating": 8.5}),
                new_data: json!({"rating": 7.2}),
                final_data: json!({"rating": 8.5}),
                resolved_at: Utc::now(),
            },
        ];

        repo.record_conflict_resolution(sync_id, &conflicts).await.unwrap();

        // Verify conflicts recorded
        let entry = sqlx::query!(
            "SELECT sync_metadata FROM list_sync_history WHERE id = $1",
            sync_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let metadata = entry.sync_metadata.unwrap();
        let conflict_resolutions = &metadata["conflict_resolutions"];
        
        assert_eq!(conflict_resolutions.as_array().unwrap().len(), 2);
        assert_eq!(conflict_resolutions[0]["conflict_type"].as_str().unwrap(), "duplicate_title");
        assert_eq!(conflict_resolutions[1]["resolution_strategy"].as_str().unwrap(), "keep_existing");

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_sync_history_pagination() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        // Create multiple sync entries
        for i in 0..25 {
            let sync_id = repo.start_sync(list_id, None).await.unwrap();
            repo.complete_sync(sync_id, "success", i, i/2, i/3, 0, i/4, None, None).await.unwrap();
        }

        // Test pagination
        let first_page = repo.get_sync_history(Some(list_id), 10, 0).await.unwrap();
        let second_page = repo.get_sync_history(Some(list_id), 10, 10).await.unwrap();
        let third_page = repo.get_sync_history(Some(list_id), 10, 20).await.unwrap();

        assert_eq!(first_page.len(), 10);
        assert_eq!(second_page.len(), 10);
        assert_eq!(third_page.len(), 5);

        // Verify order (most recent first)
        assert!(first_page[0].started_at >= first_page[9].started_at);

        // Test filtering by list_id
        let other_list_id = Uuid::new_v4();
        let other_sync_id = repo.start_sync(other_list_id, None).await.unwrap();
        repo.complete_sync(other_sync_id, "success", 1, 1, 0, 0, 0, None, None).await.unwrap();

        let filtered_history = repo.get_sync_history(Some(other_list_id), 10, 0).await.unwrap();
        assert_eq!(filtered_history.len(), 1);
        assert_eq!(filtered_history[0].import_list_id, other_list_id);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_sync_statistics_calculation() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        // Create mix of successful and failed syncs
        let now = Utc::now();
        
        // 7 successful syncs
        for i in 0..7 {
            let sync_id = repo.start_sync(list_id, None).await.unwrap();
            
            // Manually set started_at to test time ranges
            sqlx::query!(
                "UPDATE list_sync_history SET started_at = $2 WHERE id = $1",
                sync_id,
                now - Duration::hours(i * 6) // Spread over different times
            ).execute(&pool).await.unwrap();
            
            repo.complete_sync(sync_id, "success", (i+1)*10, (i+1)*5, i*2, 0, i, None, None).await.unwrap();
        }

        // 3 failed syncs
        for i in 0..3 {
            let sync_id = repo.start_sync(list_id, None).await.unwrap();
            repo.complete_sync(
                sync_id, 
                "failed", 
                0, 0, 0, 0, 0, 
                Some(format!("Error type {}", i)),
                None
            ).await.unwrap();
        }

        let stats = repo.get_sync_statistics(Some(list_id), 7).await.unwrap();

        assert_eq!(stats.total_syncs, 10);
        assert_eq!(stats.successful_syncs, 7);
        assert_eq!(stats.failed_syncs, 3);
        assert_eq!(stats.error_rate_percent, 30.0);
        assert!(stats.average_duration_ms > 0.0);
        assert_eq!(stats.total_items_processed, 280); // Sum of items_found
        assert_eq!(stats.average_items_per_sync, 28.0);
        assert!(stats.peak_items_per_second > 0.0);
        
        // Should have error statistics
        assert!(!stats.most_common_errors.is_empty());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_performance_trends() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        let now = Utc::now();

        // Create syncs at different hours with varying performance
        for hour in 0..5 {
            for sync_num in 0..3 {
                let sync_id = repo.start_sync(list_id, None).await.unwrap();
                
                // Set different hours
                sqlx::query!(
                    "UPDATE list_sync_history SET started_at = $2 WHERE id = $1",
                    sync_id,
                    now - Duration::hours(hour) + Duration::minutes(sync_num * 15)
                ).execute(&pool).await.unwrap();
                
                let items = (hour + 1) * 20;
                let duration = 2000 + (hour * 500); // Varying durations
                
                // Complete with different durations
                sqlx::query!(
                    r#"
                    UPDATE list_sync_history 
                    SET sync_status = 'success', completed_at = $2, duration_ms = $3,
                        items_found = $4
                    WHERE id = $1
                    "#,
                    sync_id,
                    now - Duration::hours(hour) + Duration::minutes(sync_num * 15) + Duration::milliseconds(duration),
                    duration as i32,
                    items as i32
                ).execute(&pool).await.unwrap();
            }
        }

        let trends = repo.get_performance_trends(Some(list_id), 6).await.unwrap();
        
        assert!(!trends.is_empty());
        
        // Verify trend data structure
        for (timestamp, avg_duration, avg_items_per_second) in &trends {
            assert!(timestamp <= &now);
            assert!(*avg_duration > 0.0);
            assert!(*avg_items_per_second > 0.0);
        }

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_active_syncs_management() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        // Start multiple syncs but don't complete them
        let sync1 = repo.start_sync(list_id, None).await.unwrap();
        let sync2 = repo.start_sync(list_id, None).await.unwrap();
        let sync3 = repo.start_sync(list_id, None).await.unwrap();

        // Get active syncs
        let active = repo.get_active_syncs().await.unwrap();
        assert_eq!(active.len(), 3);
        
        let active_ids: Vec<Uuid> = active.iter().map(|s| s.id).collect();
        assert!(active_ids.contains(&sync1));
        assert!(active_ids.contains(&sync2));
        assert!(active_ids.contains(&sync3));

        // Complete one sync
        repo.complete_sync(sync1, "success", 10, 5, 2, 0, 3, None, None).await.unwrap();

        // Cancel another
        let cancelled = repo.cancel_sync(sync2).await.unwrap();
        assert!(cancelled);

        // Check active syncs again
        let active_after = repo.get_active_syncs().await.unwrap();
        assert_eq!(active_after.len(), 1);
        assert_eq!(active_after[0].id, sync3);

        // Try cancelling already completed sync
        let not_cancelled = repo.cancel_sync(sync1).await.unwrap();
        assert!(!not_cancelled);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_cleanup_old_history() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        let now = Utc::now();

        // Create old syncs (older than retention period)
        for i in 0..5 {
            let sync_id = repo.start_sync(list_id, None).await.unwrap();
            
            // Set to 60+ days ago
            sqlx::query!(
                "UPDATE list_sync_history SET started_at = $2 WHERE id = $1",
                sync_id,
                now - Duration::days(60 + i)
            ).execute(&pool).await.unwrap();
            
            repo.complete_sync(sync_id, "success", 10, 5, 2, 0, 3, None, None).await.unwrap();
        }

        // Create recent syncs (within retention period)
        for i in 0..3 {
            let sync_id = repo.start_sync(list_id, None).await.unwrap();
            
            // Set to within last 30 days
            sqlx::query!(
                "UPDATE list_sync_history SET started_at = $2 WHERE id = $1",
                sync_id,
                now - Duration::days(i)
            ).execute(&pool).await.unwrap();
            
            repo.complete_sync(sync_id, "success", 10, 5, 2, 0, 3, None, None).await.unwrap();
        }

        // Verify we have 8 total syncs
        let all_history = repo.get_sync_history(None, 100, 0).await.unwrap();
        assert_eq!(all_history.len(), 8);

        // Cleanup syncs older than 30 days
        let deleted_count = repo.cleanup_old_history(30).await.unwrap();
        assert_eq!(deleted_count, 5);

        // Verify only recent syncs remain
        let remaining_history = repo.get_sync_history(None, 100, 0).await.unwrap();
        assert_eq!(remaining_history.len(), 3);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires test database
    async fn test_sync_with_metadata() {
        let pool = create_test_pool().await;
        cleanup_test_data(&pool).await;
        
        let repo = PostgresListSyncRepository::new(pool.clone());
        let list_id = Uuid::new_v4();

        let metadata = json!({
            "source_type": "imdb",
            "list_url": "https://www.imdb.com/chart/top",
            "user_config": {
                "min_rating": 7.0,
                "max_items": 250
            },
            "request_id": "req-123456"
        });

        let sync_id = repo.start_sync(list_id, Some(metadata.clone())).await.unwrap();

        // Verify metadata stored
        let entry = sqlx::query!(
            "SELECT sync_metadata FROM list_sync_history WHERE id = $1",
            sync_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(entry.sync_metadata.unwrap(), metadata);

        cleanup_test_data(&pool).await;
    }
}