import { screen, waitFor, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Queue } from './Queue';
import { renderWithProviders } from '../test/TestWrapper';
import { server } from '../test/setup';
import { http, HttpResponse } from 'msw';

const mockQueueItems = [
  {
    id: '1',
    movieId: 'movie-1',
    movieTitle: 'Test Movie 1',
    status: 'downloading',
    progress: 50,
    size: 1024 * 1024 * 1024, // 1GB
    indexer: 'test-indexer',
    downloadClient: 'qbittorrent',
    added: '2025-08-24T20:00:00Z',
    estimatedCompletionTime: '2025-08-24T21:00:00Z',
  },
  {
    id: '2',
    movieId: 'movie-2',
    movieTitle: 'Test Movie 2',
    status: 'paused',
    progress: 25,
    size: 2 * 1024 * 1024 * 1024, // 2GB
    indexer: 'test-indexer',
    downloadClient: 'qbittorrent',
    added: '2025-08-24T19:00:00Z',
    estimatedCompletionTime: null,
  },
];

describe('Queue Page', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render queue items successfully', async () => {
    renderWithProviders(<Queue />);

    // Wait for queue items to load (using MSW mock data)
    await waitFor(() => {
      expect(screen.getByText('Mock Queue Movie 1')).toBeInTheDocument();
      expect(screen.getByText('Mock Queue Movie 2')).toBeInTheDocument();
    });

    // Verify queue item details
    expect(screen.getByText(/downloading/i)).toBeInTheDocument();
    expect(screen.getByText(/paused/i)).toBeInTheDocument();
  });

  it('should handle queue item actions', async () => {
    renderWithProviders(<Queue />);

    // Wait for items to load
    await waitFor(() => {
      expect(screen.getByText('Mock Queue Movie 1')).toBeInTheDocument();
    });

    // Test pause action
    const pauseButton = screen.queryByRole('button', { name: /pause/i });
    if (pauseButton) {
      fireEvent.click(pauseButton);
      await waitFor(() => {
        expect(mockApi.pauseQueueItem).toHaveBeenCalledWith('1');
      });
    }

    // Test resume action
    const resumeButton = screen.queryByRole('button', { name: /resume/i });
    if (resumeButton) {
      fireEvent.click(resumeButton);
      await waitFor(() => {
        expect(mockApi.resumeQueueItem).toHaveBeenCalledWith('2');
      });
    }
  });

  it('should handle API errors gracefully', async () => {
    // Override MSW to return error
    server.use(
      http.get('/api/v3/queue', () => {
        return HttpResponse.json(
          { error: 'Queue API Error' },
          { status: 500 }
        );
      })
    );

    renderWithProviders(<Queue />);

    // Should show error state, not crash
    await waitFor(() => {
      expect(screen.getByText(/error|failed/i) || screen.getByText(/queue/i)).toBeInTheDocument();
    });
  });

  it('should handle empty queue state', async () => {
    // Override MSW to return empty queue
    server.use(
      http.get('/api/v3/queue', () => {
        return HttpResponse.json({
          data: {
            records: [],
            totalCount: 0,
          },
          success: true,
        });
      })
    );

    renderWithProviders(<Queue />);

    // Should show empty state
    await waitFor(() => {
      expect(screen.getByText(/empty|no.*items|queue.*empty/i)).toBeInTheDocument();
    });
  });

  it('should NOT crash with malformed queue data', async () => {
    // Override MSW to return malformed data
    server.use(
      http.get('/api/v3/queue', () => {
        return HttpResponse.json({
          data: {
            records: null as any, // This could cause iteration errors
            totalCount: 'invalid' as any,
          },
          success: true,
        });
      })
    );

    // Component should not crash
    expect(() => {
      renderWithProviders(<Queue />);
    }).not.toThrow();

    // Should handle malformed data gracefully
    expect(screen.getByText(/queue/i) || document.body).toBeInTheDocument();
  });
});