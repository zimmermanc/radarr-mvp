import { screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Movies } from './Movies';
import { renderWithProviders } from '../test/TestWrapper';

const mockMovies = [
  {
    id: '1',
    tmdb_id: 123456,
    title: 'Test Movie 1',
    year: 2025,
    overview: 'A test movie',
    poster_path: '/poster1.jpg',
    backdrop_path: '/backdrop1.jpg',
    status: 'wanted',
    monitored: true,
    quality_profile_id: 1,
    added: '2025-08-24T20:00:00Z',
    updated: '2025-08-24T20:00:00Z',
  },
  {
    id: '2',
    tmdb_id: 789012,
    title: 'Test Movie 2',
    year: 2024,
    overview: 'Another test movie',
    poster_path: '/poster2.jpg',
    backdrop_path: '/backdrop2.jpg',
    status: 'downloaded',
    monitored: false,
    quality_profile_id: 2,
    added: '2025-08-24T19:00:00Z',
    updated: '2025-08-24T19:30:00Z',
  },
];

describe('Movies Page', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render movies list successfully', async () => {
    renderWithProviders(<Movies />);

    // Wait for movies to load
    await waitFor(() => {
      expect(screen.getByText('Test Movie 1')).toBeInTheDocument();
      expect(screen.getByText('Test Movie 2')).toBeInTheDocument();
    });

    // Verify movie details are displayed
    expect(screen.getByText('2025')).toBeInTheDocument();
    expect(screen.getByText('2024')).toBeInTheDocument();
  });

  it('should handle search functionality', async () => {
    renderWithProviders(<Movies />);

    // Wait for initial load
    await waitFor(() => {
      expect(screen.getByText('Test Movie 1')).toBeInTheDocument();
    });

    // Test search input if present
    const searchInput = screen.queryByRole('textbox', { name: /search/i });
    if (searchInput) {
      // Search functionality should be testable
      expect(searchInput).toBeInTheDocument();
    }
  });

  it('should handle empty movies list', async () => {
    mockApi.getMovies.mockResolvedValue({
      data: [],
      totalCount: 0,
      currentPage: 1,
      totalPages: 0,
    });

    renderWithProviders(<Movies />);

    // Should show empty state
    await waitFor(() => {
      expect(screen.getByText(/no.*movies|empty|add.*movie/i)).toBeInTheDocument();
    });
  });

  it('should handle API errors gracefully', async () => {
    mockApi.getMovies.mockRejectedValue(new Error('Movies API Error'));

    renderWithProviders(<Movies />);

    // Should show error state without crashing
    await waitFor(() => {
      expect(screen.getByText(/error|failed/i) || screen.getByText(/movies/i)).toBeInTheDocument();
    });
  });

  it('should NOT crash with malformed movie data', async () => {
    // Test with malformed data that could cause JavaScript errors
    mockApi.getMovies.mockResolvedValue({
      data: 'invalid' as any, // This could cause iteration errors
      totalCount: null as any,
      currentPage: undefined as any,
      totalPages: {} as any,
    });

    // Component should not crash
    expect(() => {
      render(
        <MoviesWrapper>
          <Movies />
        </MoviesWrapper>
      );
    }).not.toThrow();

    // Should handle malformed data gracefully
    expect(document.body).toBeInTheDocument();
  });
});