// Streaming Service Types

export type MediaType = 'movie' | 'tv';
export type TimeWindow = 'day' | 'week';
export type TrendingSource = 'tmdb' | 'trakt' | 'aggregated';
export type ServiceType = 'subscription' | 'rent' | 'buy' | 'free' | 'ads';
export type VideoQuality = 'SD' | 'HD' | '4K' | 'HDR';

export interface TrendingEntry {
  id?: string;
  tmdb_id: number;
  media_type: MediaType;
  title: string;
  release_date?: string;
  poster_path?: string;
  backdrop_path?: string;
  overview?: string;
  source: TrendingSource;
  time_window: TimeWindow;
  rank?: number;
  score?: number;
  vote_average?: number;
  vote_count?: number;
  popularity?: number;
}

export interface AvailabilityItem {
  id?: string;
  tmdb_id: number;
  media_type: MediaType;
  region: string;
  service_name: string;
  service_type: ServiceType;
  service_logo_url?: string;
  deep_link?: string;
  price_amount?: number;
  price_currency?: string;
  quality?: VideoQuality;
  leaving_date?: string;
}

export interface ComingSoon {
  id?: string;
  tmdb_id: number;
  media_type: MediaType;
  title: string;
  release_date: string;
  poster_path?: string;
  backdrop_path?: string;
  overview?: string;
  source: string;
  region: string;
  streaming_services: string[];
}

export interface StreamingProvider {
  name: string;
  logo_url?: string;
  service_types: ServiceType[];
  regions: string[];
}

// API Request/Response Types
export interface TrendingQuery {
  window?: TimeWindow;
  source?: TrendingSource;
  limit?: number;
}

export interface TrendingResponse {
  entries: TrendingEntry[];
  window: TimeWindow;
  source: TrendingSource;
  fetched_at: string;
}

export interface AvailabilityResponse {
  tmdb_id: number;
  media_type: MediaType;
  region: string;
  items: AvailabilityItem[];
  fetched_at: string;
}

export interface ComingSoonResponse {
  media_type: MediaType;
  region: string;
  releases: ComingSoon[];
  fetched_at: string;
}

export interface ProvidersResponse {
  providers: StreamingProvider[];
  region: string;
}