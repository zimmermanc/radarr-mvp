#!/usr/bin/env python3
"""
Import HDBits analysis results into PostgreSQL for long-term storage and analysis
"""

import json
import psycopg2
from psycopg2.extras import RealDictCursor, Json
import sys
import os
from datetime import datetime
import argparse
import logging

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class AnalysisImporter:
    def __init__(self, db_url):
        """Initialize with database connection"""
        self.db_url = db_url
        self.conn = None
        self.session_id = None
        
    def connect(self):
        """Connect to PostgreSQL"""
        try:
            self.conn = psycopg2.connect(self.db_url)
            logger.info("Connected to PostgreSQL")
            return True
        except Exception as e:
            logger.error(f"Failed to connect to database: {e}")
            return False
    
    def disconnect(self):
        """Close database connection"""
        if self.conn:
            self.conn.close()
            logger.info("Disconnected from PostgreSQL")
    
    def create_analysis_session(self, session_type="segmented", source="hdbits"):
        """Create a new analysis session record"""
        try:
            with self.conn.cursor() as cur:
                cur.execute("""
                    INSERT INTO analysis_sessions (
                        session_type, source, started_at, status
                    ) VALUES (%s, %s, %s, %s)
                    RETURNING id
                """, (session_type, source, datetime.now(), 'running'))
                
                self.session_id = cur.fetchone()[0]
                self.conn.commit()
                logger.info(f"Created analysis session: {self.session_id}")
                return self.session_id
        except Exception as e:
            logger.error(f"Failed to create analysis session: {e}")
            self.conn.rollback()
            return None
    
    def update_analysis_session(self, pages=0, releases=0, groups=0, new_groups=0, status='completed'):
        """Update the analysis session with results"""
        if not self.session_id:
            return
        
        try:
            with self.conn.cursor() as cur:
                cur.execute("""
                    UPDATE analysis_sessions
                    SET completed_at = %s,
                        status = %s,
                        pages_processed = %s,
                        releases_analyzed = %s,
                        groups_discovered = %s,
                        new_groups_added = %s,
                        runtime_seconds = EXTRACT(EPOCH FROM (%s - started_at))
                    WHERE id = %s
                """, (datetime.now(), status, pages, releases, groups, new_groups, 
                      datetime.now(), self.session_id))
                self.conn.commit()
                logger.info(f"Updated analysis session {self.session_id}")
        except Exception as e:
            logger.error(f"Failed to update analysis session: {e}")
            self.conn.rollback()
    
    def import_scene_group(self, group_name, metrics):
        """Import or update a scene group"""
        try:
            with self.conn.cursor() as cur:
                # Check if group exists
                cur.execute("SELECT id FROM scene_groups WHERE name = %s", (group_name,))
                result = cur.fetchone()
                
                if result:
                    group_id = result[0]
                    # Update existing group
                    cur.execute("""
                        UPDATE scene_groups
                        SET last_seen = %s,
                            last_analyzed = %s,
                            metadata = metadata || %s
                        WHERE id = %s
                        RETURNING id
                    """, (datetime.now(), datetime.now(), Json(metrics), group_id))
                else:
                    # Insert new group
                    group_type = 'internal' if metrics.get('is_internal', False) else 'scene'
                    cur.execute("""
                        INSERT INTO scene_groups (name, group_type, metadata)
                        VALUES (%s, %s, %s)
                        RETURNING id
                    """, (group_name, group_type, Json(metrics)))
                    group_id = cur.fetchone()[0]
                
                # Insert metrics record
                cur.execute("""
                    INSERT INTO scene_group_metrics (
                        scene_group_id, analysis_date, release_count, 
                        internal_release_count, total_size_gb, average_size_gb,
                        freeleech_count, freeleech_percentage
                    ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
                    ON CONFLICT (scene_group_id, analysis_date) 
                    DO UPDATE SET
                        release_count = EXCLUDED.release_count,
                        internal_release_count = EXCLUDED.internal_release_count,
                        total_size_gb = EXCLUDED.total_size_gb,
                        average_size_gb = EXCLUDED.average_size_gb,
                        freeleech_count = EXCLUDED.freeleech_count,
                        freeleech_percentage = EXCLUDED.freeleech_percentage
                """, (
                    group_id,
                    datetime.now().date(),
                    metrics.get('release_count', 0),
                    metrics.get('internal_releases', 0),
                    metrics.get('total_size_gb', 0),
                    metrics.get('average_size_gb', 0),
                    metrics.get('freeleech_count', 0),
                    metrics.get('freeleech_percentage', 0)
                ))
                
                # Calculate and update reputation score
                cur.execute("""
                    SELECT calculate_scene_group_reputation(%s)
                """, (group_id,))
                new_score = cur.fetchone()[0]
                
                cur.execute("""
                    UPDATE scene_groups
                    SET reputation_score = %s,
                        confidence_level = LEAST(100, %s::DECIMAL / 10 * 10)
                    WHERE id = %s
                """, (new_score, metrics.get('release_count', 0), group_id))
                
                # Record reputation history if session exists
                if self.session_id:
                    cur.execute("""
                        INSERT INTO scene_group_reputation_history (
                            scene_group_id, analysis_session_id, 
                            new_score, sample_size, confidence
                        ) VALUES (%s, %s, %s, %s, %s)
                    """, (
                        group_id, self.session_id, new_score,
                        metrics.get('release_count', 0),
                        min(100, metrics.get('release_count', 0) / 10 * 10)
                    ))
                
                self.conn.commit()
                return group_id
                
        except Exception as e:
            logger.error(f"Failed to import scene group {group_name}: {e}")
            self.conn.rollback()
            return None
    
    def import_release(self, release_data, group_id=None):
        """Import an individual release"""
        try:
            with self.conn.cursor() as cur:
                cur.execute("""
                    INSERT INTO release_analysis (
                        scene_group_id, release_name, info_hash, 
                        title, year, codec, resolution, source,
                        size_gb, is_internal, is_freeleech,
                        seeders, leechers, metadata
                    ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
                    ON CONFLICT (info_hash) DO NOTHING
                """, (
                    group_id,
                    release_data.get('name', ''),
                    release_data.get('info_hash'),
                    release_data.get('title'),
                    release_data.get('year'),
                    release_data.get('codec'),
                    release_data.get('resolution'),
                    release_data.get('source'),
                    release_data.get('size_gb', 0),
                    release_data.get('is_internal', False),
                    release_data.get('is_freeleech', False),
                    release_data.get('seeders', 0),
                    release_data.get('leechers', 0),
                    Json(release_data.get('metadata', {}))
                ))
                self.conn.commit()
        except Exception as e:
            logger.error(f"Failed to import release: {e}")
            self.conn.rollback()
    
    def import_json_file(self, json_file):
        """Import analysis results from JSON file"""
        logger.info(f"Importing analysis from {json_file}")
        
        try:
            with open(json_file, 'r') as f:
                data = json.load(f)
            
            # Create analysis session
            self.create_analysis_session()
            
            # Track statistics
            total_groups = 0
            new_groups = 0
            total_releases = data.get('total_releases', 0)
            pages = data.get('pages_processed', 0)
            
            # Import scene groups
            scene_groups = data.get('scene_groups', {})
            for group_name, metrics in scene_groups.items():
                logger.info(f"Importing scene group: {group_name}")
                group_id = self.import_scene_group(group_name, metrics)
                if group_id:
                    total_groups += 1
                    
                    # Import individual releases if available
                    releases = metrics.get('releases', [])
                    for release in releases:
                        self.import_release(release, group_id)
            
            # Update session
            self.update_analysis_session(
                pages=pages,
                releases=total_releases,
                groups=total_groups,
                new_groups=new_groups,
                status='completed'
            )
            
            logger.info(f"Import complete: {total_groups} groups, {total_releases} releases")
            return True
            
        except Exception as e:
            logger.error(f"Failed to import JSON file: {e}")
            if self.session_id:
                self.update_analysis_session(status='failed')
            return False
    
    def import_csv_file(self, csv_file):
        """Import scene groups from CSV file"""
        import csv
        
        logger.info(f"Importing CSV from {csv_file}")
        
        try:
            with open(csv_file, 'r') as f:
                reader = csv.DictReader(f)
                
                for row in reader:
                    group_name = row.get('group_name', row.get('scene_group', ''))
                    if group_name:
                        metrics = {
                            'release_count': int(row.get('release_count', 0)),
                            'internal_releases': int(row.get('internal_releases', 0)),
                            'total_size_gb': float(row.get('total_size_gb', 0)),
                            'reputation_score': float(row.get('reputation_score', 50))
                        }
                        self.import_scene_group(group_name, metrics)
            
            self.conn.commit()
            logger.info("CSV import complete")
            return True
            
        except Exception as e:
            logger.error(f"Failed to import CSV file: {e}")
            self.conn.rollback()
            return False
    
    def get_top_groups(self, limit=20):
        """Get top scene groups by reputation"""
        try:
            with self.conn.cursor(cursor_factory=RealDictCursor) as cur:
                cur.execute("""
                    SELECT * FROM scene_group_standings
                    LIMIT %s
                """, (limit,))
                return cur.fetchall()
        except Exception as e:
            logger.error(f"Failed to get top groups: {e}")
            return []

def main():
    parser = argparse.ArgumentParser(description='Import HDBits analysis to PostgreSQL')
    parser.add_argument('input_file', help='JSON or CSV file to import')
    parser.add_argument('--db-url', default=os.getenv('DATABASE_URL', 
                        'postgresql://radarr:radarr@localhost:5432/radarr'),
                        help='PostgreSQL connection URL')
    parser.add_argument('--show-top', type=int, metavar='N',
                        help='Show top N groups after import')
    
    args = parser.parse_args()
    
    # Create importer
    importer = AnalysisImporter(args.db_url)
    
    if not importer.connect():
        sys.exit(1)
    
    try:
        # Import based on file type
        if args.input_file.endswith('.json'):
            success = importer.import_json_file(args.input_file)
        elif args.input_file.endswith('.csv'):
            success = importer.import_csv_file(args.input_file)
        else:
            logger.error("Unsupported file type. Use .json or .csv")
            success = False
        
        if success and args.show_top:
            # Show top groups
            print("\nTop Scene Groups by Reputation:")
            print("-" * 80)
            groups = importer.get_top_groups(args.show_top)
            for group in groups:
                print(f"{group['name']:20} Score: {group['reputation_score']:5.1f} "
                      f"Confidence: {group['confidence_level']:5.1f}% "
                      f"Releases: {group['release_count'] or 0:5}")
    
    finally:
        importer.disconnect()

if __name__ == "__main__":
    main()