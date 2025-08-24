import axios, { type AxiosInstance } from 'axios';
import type {
  MediaType,
  TrendingQuery,
  TrendingResponse,
  AvailabilityResponse,
  ComingSoonResponse,
  ProvidersResponse,
} from '../types/streaming';

class StreamingApiClient {
  private client: AxiosInstance;

  constructor(baseUrl: string, apiKey: string) {
    this.client = axios.create({
      baseURL: `${baseUrl}/api/streaming`,
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json',
        'X-Api-Key': apiKey,
      },
    });

    // Add response interceptor for error handling
    this.client.interceptors.response.use(
      (response) => response,
      (error) => {
        console.error('Streaming API Error:', error);
        return Promise.reject(error);
      }
    );
  }

  // Get trending movies or TV shows
  async getTrending(
    mediaType: MediaType,
    params?: TrendingQuery
  ): Promise<TrendingResponse> {
    const response = await this.client.get(`/trending/${mediaType}`, { params });
    return response.data.data;
  }

  // Get streaming availability for a specific title
  async getAvailability(
    tmdbId: number,
    region: string = 'US'
  ): Promise<AvailabilityResponse> {
    const response = await this.client.get(`/availability/${tmdbId}`, {
      params: { region },
    });
    return response.data.data;
  }

  // Get upcoming releases
  async getComingSoon(
    mediaType: MediaType,
    region: string = 'US'
  ): Promise<ComingSoonResponse> {
    const response = await this.client.get(`/coming-soon/${mediaType}`, {
      params: { region },
    });
    return response.data.data;
  }

  // Get list of streaming providers
  async getProviders(region: string = 'US'): Promise<ProvidersResponse> {
    const response = await this.client.get('/providers', {
      params: { region },
    });
    return response.data.data;
  }

  // Force cache refresh
  async refreshCache(): Promise<void> {
    await this.client.post('/cache/refresh');
  }

  // Initialize Trakt OAuth
  async initTraktAuth(): Promise<{
    device_code: string;
    user_code: string;
    verification_url: string;
    expires_in: number;
    interval: number;
  }> {
    const response = await this.client.post('/trakt/auth/init');
    return response.data.data;
  }
}

// Create singleton instance
let streamingApiInstance: StreamingApiClient | null = null;

export function initStreamingApi(baseUrl: string, apiKey: string): StreamingApiClient {
  if (!streamingApiInstance) {
    streamingApiInstance = new StreamingApiClient(baseUrl, apiKey);
  }
  return streamingApiInstance;
}

export function getStreamingApi(): StreamingApiClient {
  if (!streamingApiInstance) {
    throw new Error('Streaming API not initialized. Call initStreamingApi first.');
  }
  return streamingApiInstance;
}

export default StreamingApiClient;