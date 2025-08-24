// API Response Types
export interface HealthResponse {
  status: string;
  timestamp: string;
  version?: string;
}

export interface Movie {
  id: number;
  title: string;
  original_title?: string;
  year: number;
  tmdb_id: number;
  imdb_id?: string;
  overview?: string;
  poster_path?: string;
  backdrop_path?: string;
  release_date?: string;
  runtime?: number;
  genres?: string[];
  vote_average?: number;
  vote_count?: number;
  popularity?: number;
  status?: 'announced' | 'in_cinemas' | 'released' | 'deleted';
  monitored?: boolean;
  has_file?: boolean;
  file_path?: string;
  quality_profile_id?: number;
  tags?: string[];
  added_date?: string;
  updated_date?: string;
}

export interface MoviesResponse {
  movies: Movie[];
  total: number;
  page?: number;
  limit?: number;
}

export interface QualityProfile {
  id: number;
  name: string;
  cutoff: number;
  items: QualityProfileItem[];
  min_format_score: number;
  cutoff_format_score: number;
  format_items: FormatItem[];
}

export interface QualityProfileItem {
  id: number;
  name: string;
  quality: Quality;
  allowed: boolean;
}

export interface Quality {
  id: number;
  name: string;
  source: string;
  resolution: number;
}

export interface FormatItem {
  format: CustomFormat;
  score: number;
}

export interface CustomFormat {
  id: number;
  name: string;
  specifications: any[];
}

export interface SearchResult {
  title: string;
  year: number;
  tmdb_id: number;
  imdb_id?: string;
  overview?: string;
  poster_path?: string;
  release_date?: string;
  vote_average?: number;
  popularity?: number;
}

export interface QueueItem {
  id: string;
  movieId: number;
  movieTitle: string;
  quality: string;
  protocol: 'torrent' | 'usenet';
  indexer: string;
  downloadClient: string;
  status: 'queued' | 'downloading' | 'paused' | 'completed' | 'failed' | 'importing';
  size: number;
  sizeLeft: number;
  timeleft?: string;
  estimatedCompletionTime?: string;
  downloadedSize: number;
  progress: number;
  downloadRate?: number;
  uploadRate?: number;
  seeders?: number;
  leechers?: number;
  eta?: string;
  errorMessage?: string;
  trackedDownloadStatus?: string;
  trackedDownloadState?: string;
  statusMessages?: string[];
  outputPath?: string;
  downloadId?: string;
  added: string;
}

export interface QueueResponse {
  items: QueueItem[];
  totalRecords: number;
  page?: number;
  pageSize?: number;
}

export interface ApiError {
  error: string;
  message: string;
  details?: any;
  timestamp: string;
}

// Request Types
export interface AddMovieRequest {
  tmdb_id: number;
  quality_profile_id: number;
  monitored?: boolean;
  search_for_movie?: boolean;
  root_folder_path?: string;
  tags?: string[];
}

export interface UpdateMovieRequest {
  title?: string;
  monitored?: boolean;
  quality_profile_id?: number;
  tags?: string[];
}

export interface SearchMovieRequest {
  term: string;
  year?: number;
  limit?: number;
  offset?: number;
}

export interface SearchRelease {
  id: string;
  title: string;
  indexer: string;
  indexerId: string;
  size: number;
  quality: string;
  resolution?: string;
  source?: string;
  codec?: string;
  seeders: number;
  leechers: number;
  sceneGroup?: string;
  releaseGroup?: string;
  languages?: string[];
  publishDate: string;
  downloadUrl?: string;
  infoUrl?: string;
  score?: number;
  matchType?: 'exact' | 'partial' | 'fuzzy';
}

export interface SearchReleasesRequest {
  movieId: number;
  indexers?: string[];
}

export interface DownloadReleaseRequest {
  movieId: number;
  releaseId: string;
  indexerId: string;
  downloadUrl: string;
}

export interface BulkUpdateRequest {
  movieIds: number[];
  updates: {
    monitored?: boolean;
    qualityProfileId?: number;
    tags?: string[];
  };
}

// Utility Types
export type ApiResponse<T> = {
  data: T;
  success: true;
} | {
  error: ApiError;
  success: false;
};

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  limit: number;
  has_next: boolean;
  has_previous: boolean;
}

export interface ApiConfig {
  baseUrl: string;
  apiKey: string;
  timeout?: number;
}

// Filter and Sort Types
export type MovieSortField = 'title' | 'year' | 'rating' | 'added_date' | 'release_date';
export type SortDirection = 'asc' | 'desc';

export interface MovieFilters {
  monitored?: boolean;
  status?: Movie['status'];
  has_file?: boolean;
  quality_profile_id?: number;
  search?: string;
  tags?: string[];
}

export interface MovieListParams {
  sort_by?: MovieSortField;
  sort_direction?: SortDirection;
  page?: number;
  limit?: number;
  filters?: MovieFilters;
}