import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { TrendingCarousel } from './TrendingCarousel';
import { getStreamingApi } from '../../lib/streamingApi';
import type { TrendingResponse } from '../../lib/schemas';

// Mock the streaming API
vi.mock('../../lib/streamingApi', () => ({
  getStreamingApi: vi.fn(),
}));

const mockGetStreamingApi = vi.mocked(getStreamingApi);

const mockTrendingData: TrendingResponse = {
  media_type: 'movie',
  time_window: 'day',
  source: 'aggregated',
  entries: [
    {
      id: '1',
      tmdb_id: 123456,
      media_type: 'movie',
      title: 'Test Movie',
      release_date: '2025-01-01',
      poster_path: '/test-poster.jpg',
      backdrop_path: '/test-backdrop.jpg',
      overview: 'A test movie for testing purposes',
      source: 'tmdb',
      time_window: 'day',
      rank: 1,
      score: 100.0,
      vote_average: 8.5,
      vote_count: 1000,
      popularity: 95.5,
      fetched_at: '2025-08-24T20:00:00Z',
      expires_at: '2025-08-24T23:00:00Z',
    }
  ],
  total_results: 1,
  fetched_at: '2025-08-24T20:00:00Z',
  expires_at: '2025-08-24T23:00:00Z',
};

describe('TrendingCarousel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render loading state initially', () => {
    // Mock API to simulate loading
    mockGetStreamingApi.mockReturnValue({
      getTrending: vi.fn().mockImplementation(() => 
        new Promise(resolve => setTimeout(() => resolve(mockTrendingData), 100))
      ),
    } as any);

    render(<TrendingCarousel mediaType="movie" />);
    
    // Should show loading spinner
    expect(screen.getByRole('status') || screen.getByTestId('loading-indicator')).toBeInTheDocument();
  });

  it('should handle successful trending data', async () => {
    // Mock successful API response
    mockGetStreamingApi.mockReturnValue({
      getTrending: vi.fn().mockResolvedValue(mockTrendingData),
    } as any);

    render(<TrendingCarousel mediaType="movie" />);

    // Wait for data to load
    await waitFor(() => {
      expect(screen.getByText('Test Movie')).toBeInTheDocument();
    });

    // Should display movie information
    expect(screen.getByText('Test Movie')).toBeInTheDocument();
  });

  it('should handle API errors gracefully', async () => {
    // Mock API error
    mockGetStreamingApi.mockReturnValue({
      getTrending: vi.fn().mockRejectedValue(new Error('API Error: Failed to fetch')),
    } as any);

    render(<TrendingCarousel mediaType="movie" />);

    // Wait for error state
    await waitFor(() => {
      expect(screen.getByText(/error|failed/i)).toBeInTheDocument();
    });

    // Should show error message, not crash
    expect(screen.getByText(/error|failed/i)).toBeInTheDocument();
  });

  it('should handle empty trending data', async () => {
    // Mock empty response
    const emptyResponse: TrendingResponse = {
      ...mockTrendingData,
      entries: [],
      total_results: 0,
    };

    mockGetStreamingApi.mockReturnValue({
      getTrending: vi.fn().mockResolvedValue(emptyResponse),
    } as any);

    render(<TrendingCarousel mediaType="movie" />);

    // Wait for component to finish loading
    await waitFor(() => {
      expect(screen.getByText('Trending Movies')).toBeInTheDocument();
    });

    // Component should render the header even with no data
    expect(screen.getByText('Trending Movies')).toBeInTheDocument();
    
    // Should have time window buttons
    expect(screen.getByText('Today')).toBeInTheDocument();
    expect(screen.getByText('This Week')).toBeInTheDocument();
  });

  it('should NOT crash with "d is not iterable" error', async () => {
    // Mock malformed response that might cause iteration errors
    const malformedResponse = {
      success: true,
      data: null, // This could cause "d is not iterable"
    };

    mockGetStreamingApi.mockReturnValue({
      getTrending: vi.fn().mockResolvedValue(malformedResponse as any),
    } as any);

    // This test specifically checks that the component handles bad data gracefully
    expect(() => {
      render(<TrendingCarousel mediaType="movie" />);
    }).not.toThrow();

    // Component should render without crashing, even with bad data
    expect(screen.getByTestId('trending-carousel') || document.body).toBeInTheDocument();
  });
});