import { http, HttpResponse } from 'msw';

// Mock data
const mockTrendingMovies = {
  success: true,
  data: {
    media_type: 'movie',
    time_window: 'day',
    source: 'aggregated',
    entries: [
      {
        id: '1',
        tmdb_id: 123456,
        media_type: 'movie',
        title: 'Mock Trending Movie 1',
        release_date: '2025-01-01',
        poster_path: '/mock-poster1.jpg',
        backdrop_path: '/mock-backdrop1.jpg',
        overview: 'A mock movie for testing',
        source: 'tmdb',
        time_window: 'day',
        rank: 1,
        score: 100.0,
        vote_average: 8.5,
        vote_count: 1000,
        popularity: 95.5,
        fetched_at: '2025-08-24T20:00:00Z',
        expires_at: '2025-08-24T23:00:00Z',
      },
      {
        id: '2',
        tmdb_id: 789012,
        media_type: 'movie',
        title: 'Mock Trending Movie 2',
        release_date: '2025-02-14',
        poster_path: '/mock-poster2.jpg',
        backdrop_path: '/mock-backdrop2.jpg',
        overview: 'Another mock movie for testing',
        source: 'tmdb',
        time_window: 'day',
        rank: 2,
        score: 92.5,
        vote_average: 7.8,
        vote_count: 850,
        popularity: 88.2,
        fetched_at: '2025-08-24T20:00:00Z',
        expires_at: '2025-08-24T23:00:00Z',
      },
    ],
    total_results: 2,
    fetched_at: '2025-08-24T20:00:00Z',
    expires_at: '2025-08-24T23:00:00Z',
  },
  error: null,
  message: null,
};

const mockQueueItems = {
  records: [
      {
        id: '1',
        movieId: 'movie-1',
        movieTitle: 'Mock Queue Movie 1',
        status: 'downloading',
        progress: 75,
        size: 4 * 1024 * 1024 * 1024,
        indexer: 'Mock Indexer',
        downloadClient: 'qBittorrent',
        added: '2025-08-24T20:00:00Z',
        estimatedCompletionTime: '2025-08-24T20:30:00Z',
      },
      {
        id: '2',
        movieId: 'movie-2',
        movieTitle: 'Mock Queue Movie 2',
        status: 'paused',
        progress: 45,
        size: 6 * 1024 * 1024 * 1024,
        indexer: 'Mock Indexer',
        downloadClient: 'qBittorrent',
        added: '2025-08-24T19:30:00Z',
        estimatedCompletionTime: null,
      },
    ],
    totalCount: 2,
};

const mockMovies = {
  data: {
    movies: [
      {
        id: '1',
        tmdb_id: 123456,
        title: 'Mock Movie 1',
        year: 2025,
        overview: 'A mock movie for testing',
        poster_path: '/mock-poster1.jpg',
        backdrop_path: '/mock-backdrop1.jpg',
        status: 'wanted',
        monitored: true,
        quality_profile_id: 1,
        added: '2025-08-24T20:00:00Z',
        updated: '2025-08-24T20:00:00Z',
      },
    ],
    totalCount: 1,
    currentPage: 1,
    totalPages: 1,
  },
  success: true,
};

export const handlers = [
  // Streaming API endpoints
  http.get('/api/streaming/trending/:mediaType/:timeWindow', ({ params }) => {
    const { mediaType, timeWindow } = params;
    return HttpResponse.json({
      ...mockTrendingMovies,
      data: {
        ...mockTrendingMovies.data,
        media_type: mediaType,
        time_window: timeWindow,
      },
    });
  }),

  http.get('/api/streaming/providers', () => {
    return HttpResponse.json({
      success: true,
      data: {
        providers: [
          {
            service_name: 'Netflix',
            region: 'US',
            logo_path: '/netflix-logo.png',
          },
          {
            service_name: 'Disney+',
            region: 'US', 
            logo_path: '/disney-logo.png',
          },
        ],
      },
    });
  }),

  http.get('/api/streaming/availability/:tmdbId', ({ params }) => {
    return HttpResponse.json({
      success: true,
      data: {
        tmdb_id: parseInt(params.tmdbId as string),
        streaming_sources: [
          {
            service_name: 'Netflix',
            region: 'US',
            url: 'https://netflix.com/title/123456',
          },
        ],
      },
    });
  }),

  // Core API endpoints  
  http.get('/api/v3/queue', () => {
    return HttpResponse.json(mockQueueItems);
  }),

  http.get('/api/v3/movie', () => {
    return HttpResponse.json(mockMovies);
  }),

  http.put('/api/v3/queue/:id/pause', ({ params }) => {
    return HttpResponse.json({ success: true });
  }),

  http.put('/api/v3/queue/:id/resume', ({ params }) => {
    return HttpResponse.json({ success: true });
  }),

  http.delete('/api/v3/queue/:id', ({ params }) => {
    return HttpResponse.json({ success: true });
  }),

  // Health check
  http.get('/health', () => {
    return HttpResponse.json({
      status: 'healthy',
      service: 'radarr-mvp',
      timestamp: new Date().toISOString(),
    });
  }),

  // Error scenarios for testing
  http.get('/api/streaming/trending/error/day', () => {
    return HttpResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }),

  http.get('/api/streaming/trending/malformed/day', () => {
    // Return malformed data to test error handling
    return HttpResponse.json({
      success: true,
      data: null, // This could cause "d is not iterable"
    });
  }),
];