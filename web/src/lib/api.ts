import axios, { AxiosError } from 'axios';
import type { AxiosInstance, AxiosResponse } from 'axios';
import type {
  HealthResponse,
  Movie,
  MoviesResponse,
  QualityProfile,
  SearchResult,
  ApiError,
  ApiResponse,
  AddMovieRequest,
  UpdateMovieRequest,
  SearchMovieRequest,
  MovieListParams,
  ApiConfig,
} from '../types/api';

class RadarrApiClient {
  private client: AxiosInstance;
  private config: ApiConfig;

  constructor(config: ApiConfig) {
    this.config = config;
    this.client = axios.create({
      baseURL: config.baseUrl,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        'X-Api-Key': config.apiKey,
      },
    });

    // Request interceptor for logging
    this.client.interceptors.request.use(
      (config) => {
        if (import.meta.env.VITE_DEBUG === 'true') {
          console.log(`🚀 API Request: ${config.method?.toUpperCase()} ${config.url}`, {
            params: config.params,
            data: config.data,
          });
        }
        return config;
      },
      (error) => {
        console.error('❌ API Request Error:', error);
        return Promise.reject(error);
      }
    );

    // Response interceptor for error handling
    this.client.interceptors.response.use(
      (response: AxiosResponse) => {
        if (import.meta.env.VITE_DEBUG === 'true') {
          console.log(`✅ API Response: ${response.status}`, response.data);
        }
        return response;
      },
      (error: AxiosError) => {
        console.error('❌ API Response Error:', error);
        
        // Transform axios error to our API error format
        const apiError: ApiError = {
          error: error.code || 'UNKNOWN_ERROR',
          message: error.message,
          details: error.response?.data || null,
          timestamp: new Date().toISOString(),
        };

        // Handle specific error cases
        if (error.response?.status === 401) {
          apiError.message = 'Invalid API key or unauthorized access';
        } else if (error.response?.status === 404) {
          apiError.message = 'Resource not found';
        } else if (error.response?.status === 429) {
          apiError.message = 'Rate limit exceeded';
        } else if (error.code === 'NETWORK_ERROR' || error.code === 'ECONNREFUSED') {
          apiError.message = 'Unable to connect to Radarr API. Please check if the service is running.';
        }

        return Promise.reject(apiError);
      }
    );
  }

  // Utility method to handle API responses
  private async handleResponse<T>(promise: Promise<AxiosResponse<T>>): Promise<ApiResponse<T>> {
    try {
      const response = await promise;
      return { data: response.data, success: true };
    } catch (error) {
      return { error: error as ApiError, success: false };
    }
  }

  // Health check
  async getHealth(): Promise<ApiResponse<HealthResponse>> {
    return this.handleResponse(this.client.get<HealthResponse>('/health'));
  }

  // Movie endpoints
  async getMovies(params?: MovieListParams): Promise<ApiResponse<MoviesResponse>> {
    const queryParams = new URLSearchParams();
    
    if (params) {
      if (params.sort_by) queryParams.append('sort_by', params.sort_by);
      if (params.sort_direction) queryParams.append('sort_direction', params.sort_direction);
      if (params.page) queryParams.append('page', params.page.toString());
      if (params.limit) queryParams.append('limit', params.limit.toString());
      
      if (params.filters) {
        if (params.filters.monitored !== undefined) {
          queryParams.append('monitored', params.filters.monitored.toString());
        }
        if (params.filters.status) queryParams.append('status', params.filters.status);
        if (params.filters.has_file !== undefined) {
          queryParams.append('has_file', params.filters.has_file.toString());
        }
        if (params.filters.quality_profile_id) {
          queryParams.append('quality_profile_id', params.filters.quality_profile_id.toString());
        }
        if (params.filters.search) queryParams.append('search', params.filters.search);
        if (params.filters.tags && params.filters.tags.length > 0) {
          params.filters.tags.forEach(tag => queryParams.append('tags', tag));
        }
      }
    }

    const url = `/api/v3/movie${queryParams.toString() ? `?${queryParams.toString()}` : ''}`;
    
    try {
      const response = await this.client.get<any>(url);
      
      // Transform the API response to match expected format
      const transformedData: MoviesResponse = {
        movies: (response.data.records || []).map((movie: any) => ({
          ...movie,
          id: movie.id,
          title: movie.title,
          year: movie.year,
          has_file: movie.has_file || false,
          monitored: movie.monitored !== undefined ? movie.monitored : true,
          tmdb_id: movie.tmdbId || movie.tmdb_id,
          imdb_id: movie.imdbId || movie.imdb_id,
          status: movie.status || 'released',
        })),
        total: response.data.totalCount || response.data.records?.length || 0,
        page: response.data.page || 1,
        limit: response.data.pageSize || params?.limit || 20
      };
      
      return { data: transformedData, success: true };
    } catch (error) {
      return { error: error as ApiError, success: false };
    }
  }

  async getMovie(id: number): Promise<ApiResponse<Movie>> {
    return this.handleResponse(this.client.get<Movie>(`/api/v3/movie/${id}`));
  }

  async addMovie(movieData: AddMovieRequest): Promise<ApiResponse<Movie>> {
    return this.handleResponse(this.client.post<Movie>('/api/v3/movie', movieData));
  }

  async updateMovie(id: number, movieData: UpdateMovieRequest): Promise<ApiResponse<Movie>> {
    return this.handleResponse(this.client.put<Movie>(`/api/v3/movie/${id}`, movieData));
  }

  async deleteMovie(id: number, deleteFiles = false, addImportExclusion = false): Promise<ApiResponse<void>> {
    const params = new URLSearchParams();
    if (deleteFiles) params.append('deleteFiles', 'true');
    if (addImportExclusion) params.append('addImportExclusion', 'true');
    
    const url = `/api/v3/movie/${id}${params.toString() ? `?${params.toString()}` : ''}`;
    return this.handleResponse(this.client.delete<void>(url));
  }

  // Search endpoints
  async searchMovies(searchData: SearchMovieRequest): Promise<ApiResponse<SearchResult[]>> {
    const params = new URLSearchParams();
    params.append('term', searchData.term);
    if (searchData.year) params.append('year', searchData.year.toString());
    if (searchData.limit) params.append('limit', searchData.limit.toString());
    if (searchData.offset) params.append('offset', searchData.offset.toString());

    return this.handleResponse(
      this.client.get<SearchResult[]>(`/api/v3/movie/lookup?${params.toString()}`)
    );
  }

  // Quality Profile endpoints
  async getQualityProfiles(): Promise<ApiResponse<QualityProfile[]>> {
    return this.handleResponse(this.client.get<QualityProfile[]>('/api/v3/qualityprofile'));
  }

  async getQualityProfile(id: number): Promise<ApiResponse<QualityProfile>> {
    return this.handleResponse(this.client.get<QualityProfile>(`/api/v3/qualityprofile/${id}`));
  }

  // Utility methods
  updateConfig(newConfig: Partial<ApiConfig>): void {
    this.config = { ...this.config, ...newConfig };
    
    // Update base URL if changed
    if (newConfig.baseUrl) {
      this.client.defaults.baseURL = newConfig.baseUrl;
    }
    
    // Update API key if changed
    if (newConfig.apiKey) {
      this.client.defaults.headers['X-Api-Key'] = newConfig.apiKey;
    }
    
    // Update timeout if changed
    if (newConfig.timeout) {
      this.client.defaults.timeout = newConfig.timeout;
    }
  }

  getConfig(): ApiConfig {
    return { ...this.config };
  }

  // Test connection
  async testConnection(): Promise<boolean> {
    try {
      const response = await this.getHealth();
      return response.success;
    } catch {
      return false;
    }
  }
}

// Create and export a singleton instance
// In development, use relative URLs that get proxied by Vite
// In production, use the configured base URL or detect from window.location
const getBaseUrl = (): string => {
  // If explicitly set, use it (for production deployments)
  if (import.meta.env.VITE_API_BASE_URL) {
    return import.meta.env.VITE_API_BASE_URL;
  }
  
  // In development with Vite dev server, use relative URLs (will be proxied)
  if (import.meta.env.DEV) {
    return ''; // Relative URLs will be proxied by Vite
  }
  
  // In production without explicit config, detect from current location
  return `${window.location.protocol}//${window.location.hostname}:7878`;
};

const apiConfig: ApiConfig = {
  baseUrl: getBaseUrl(),
  apiKey: import.meta.env.VITE_API_KEY || 'mysecurekey123',
  timeout: 30000,
};

export const radarrApi = new RadarrApiClient(apiConfig);

// Export the class for creating custom instances if needed
export { RadarrApiClient };

// Export utility functions
export const createApiClient = (config: ApiConfig): RadarrApiClient => {
  return new RadarrApiClient(config);
};

export const isApiError = (response: ApiResponse<any>): response is { error: ApiError; success: false } => {
  return !response.success;
};

export const getErrorMessage = (error: ApiError): string => {
  return error.message || error.error || 'An unknown error occurred';
};