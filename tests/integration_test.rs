use radarr_core::models::{Movie, Quality, Release};
use radarr_decision::scoring::QualityScorer;

#[test]
fn test_quality_scoring_integration() {
    let scorer = QualityScorer::new();
    
    // Test high-quality SPARKS release
    let sparks_release = Release {
        id: 1,
        title: "Inception.2010.1080p.BluRay.x264-SPARKS".to_string(),
        indexer: "HDBits".to_string(),
        quality: Quality {
            id: 1,
            name: "1080p BluRay".to_string(),
            resolution: Some("1080p".to_string()),
            source: Some("BluRay".to_string()),
            codec: Some("H.264".to_string()),
        },
        size: 10 * 1024 * 1024 * 1024, // 10GB
        seeders: Some(200),
        leechers: Some(5),
        languages: vec!["English".to_string(), "Spanish".to_string()],
        scene_group: Some("SPARKS".to_string()),
        score: None,
    };
    
    let sparks_score = scorer.score_release(&sparks_release);
    println!("SPARKS Release Score: {}", sparks_score.total_score);
    println!("  Scene Group: {}", sparks_score.scene_group_score);
    println!("  Technical: {}", sparks_score.technical_score);
    println!("  Metadata: {}", sparks_score.metadata_score);
    
    // Test lower-quality YTS release
    let yts_release = Release {
        id: 2,
        title: "Inception.2010.720p.WEBRip.x264-YTS".to_string(),
        indexer: "Public".to_string(),
        quality: Quality {
            id: 2,
            name: "720p WEBRip".to_string(),
            resolution: Some("720p".to_string()),
            source: Some("WEBRip".to_string()),
            codec: Some("H.264".to_string()),
        },
        size: 1 * 1024 * 1024 * 1024, // 1GB
        seeders: Some(50),
        leechers: Some(20),
        languages: vec!["English".to_string()],
        scene_group: Some("YTS".to_string()),
        score: None,
    };
    
    let yts_score = scorer.score_release(&yts_release);
    println!("\nYTS Release Score: {}", yts_score.total_score);
    println!("  Scene Group: {}", yts_score.scene_group_score);
    println!("  Technical: {}", yts_score.technical_score);
    println!("  Metadata: {}", yts_score.metadata_score);
    
    // SPARKS should score significantly higher than YTS
    assert!(sparks_score.total_score > yts_score.total_score);
    assert!(sparks_score.total_score > 75); // High quality threshold
    assert!(yts_score.total_score < 65);    // Lower quality threshold
}

#[test]
fn test_scene_group_preference() {
    let scorer = QualityScorer::new();
    
    // Two releases with same technical quality but different scene groups
    let flux_release = Release {
        id: 3,
        title: "Movie.2024.2160p.WEB-DL.DDP5.1.H.265-FLUX".to_string(),
        indexer: "HDBits".to_string(),
        quality: Quality {
            id: 3,
            name: "2160p WEB-DL".to_string(),
            resolution: Some("2160p".to_string()),
            source: Some("WEB-DL".to_string()),
            codec: Some("H.265".to_string()),
        },
        size: 20 * 1024 * 1024 * 1024, // 20GB
        seeders: Some(100),
        leechers: Some(10),
        languages: vec!["English".to_string()],
        scene_group: Some("FLUX".to_string()),
        score: None,
    };
    
    let unknown_release = Release {
        id: 4,
        title: "Movie.2024.2160p.WEB-DL.DDP5.1.H.265-UNKNOWN".to_string(),
        indexer: "Public".to_string(),
        quality: Quality {
            id: 3,
            name: "2160p WEB-DL".to_string(),
            resolution: Some("2160p".to_string()),
            source: Some("WEB-DL".to_string()),
            codec: Some("H.265".to_string()),
        },
        size: 20 * 1024 * 1024 * 1024, // 20GB
        seeders: Some(100),
        leechers: Some(10),
        languages: vec!["English".to_string()],
        scene_group: Some("UNKNOWN".to_string()),
        score: None,
    };
    
    let flux_score = scorer.score_release(&flux_release);
    let unknown_score = scorer.score_release(&unknown_release);
    
    println!("\nFLUX vs Unknown Group Comparison:");
    println!("FLUX Total: {}", flux_score.total_score);
    println!("Unknown Total: {}", unknown_score.total_score);
    
    // FLUX should score higher due to scene group reputation
    assert!(flux_score.total_score > unknown_score.total_score);
    assert_eq!(flux_score.scene_group_score, 100); // FLUX is top-tier
    assert_eq!(unknown_score.scene_group_score, 30); // Unknown gets default low score
}