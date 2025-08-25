import type { Meta, StoryObj } from '@storybook/react';
import { TrendingCarousel } from './TrendingCarousel';
import { within, expect, userEvent } from '@storybook/testing-library';

// Mock the streaming API for Storybook
import { getStreamingApi } from '../../lib/streamingApi';
import { vi } from 'vitest';

vi.mock('../../lib/streamingApi', () => ({
  getStreamingApi: vi.fn(),
}));

const meta: Meta<typeof TrendingCarousel> = {
  title: 'Components/Streaming/TrendingCarousel',
  component: TrendingCarousel,
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component: 'Displays trending movies and TV shows with carousel navigation.',
      },
    },
  },
  decorators: [
    (Story) => (
      <div className="p-4 bg-gray-50 dark:bg-gray-900 min-h-screen">
        <Story />
      </div>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof TrendingCarousel>;

// Mock data for stories
const mockTrendingEntries = [
  {
    id: '1',
    tmdb_id: 123456,
    media_type: 'movie' as const,
    title: 'Amazing Action Movie',
    release_date: '2025-01-01',
    poster_path: '/poster1.jpg',
    backdrop_path: '/backdrop1.jpg',
    overview: 'An incredible action-packed adventure that will keep you on the edge of your seat.',
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
    media_type: 'movie' as const,
    title: 'Romantic Comedy Hit',
    release_date: '2025-02-14',
    poster_path: '/poster2.jpg',
    backdrop_path: '/backdrop2.jpg',
    overview: 'A heartwarming romantic comedy that will make you laugh and cry.',
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
];

// Loading state story
export const Loading: Story = {
  parameters: {
    docs: {
      description: {
        story: 'Shows the loading state while fetching trending data.',
      },
    },
  },
  beforeEach: () => {
    const mockApi = {
      getTrending: vi.fn().mockImplementation(() => 
        new Promise(resolve => setTimeout(() => resolve({
          media_type: 'movie',
          time_window: 'day',
          source: 'aggregated',
          entries: mockTrendingEntries,
          total_results: 2,
          fetched_at: '2025-08-24T20:00:00Z',
          expires_at: '2025-08-24T23:00:00Z',
        }), 2000))
      ),
    };
    vi.mocked(getStreamingApi).mockReturnValue(mockApi as any);
  },
};

// Populated state story
export const WithData: Story = {
  parameters: {
    docs: {
      description: {
        story: 'Shows the carousel populated with trending movie data.',
      },
    },
  },
  beforeEach: () => {
    const mockApi = {
      getTrending: vi.fn().mockResolvedValue({
        media_type: 'movie',
        time_window: 'day',
        source: 'aggregated',
        entries: mockTrendingEntries,
        total_results: 2,
        fetched_at: '2025-08-24T20:00:00Z',
        expires_at: '2025-08-24T23:00:00Z',
      }),
    };
    vi.mocked(getStreamingApi).mockReturnValue(mockApi as any);
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Wait for trending content to load
    await expect(canvas.getByText('Amazing Action Movie')).toBeInTheDocument();
    
    // Test carousel navigation if present
    const nextButton = canvas.queryByRole('button', { name: /next|right/i });
    if (nextButton) {
      await userEvent.click(nextButton);
    }
  },
};

// Empty state story
export const Empty: Story = {
  parameters: {
    docs: {
      description: {
        story: 'Shows the empty state when no trending data is available.',
      },
    },
  },
  beforeEach: () => {
    const mockApi = {
      getTrending: vi.fn().mockResolvedValue({
        media_type: 'movie',
        time_window: 'day',
        source: 'aggregated',
        entries: [],
        total_results: 0,
        fetched_at: '2025-08-24T20:00:00Z',
        expires_at: '2025-08-24T23:00:00Z',
      }),
    };
    vi.mocked(getStreamingApi).mockReturnValue(mockApi as any);
  },
};

// Error state story
export const Error: Story = {
  parameters: {
    docs: {
      description: {
        story: 'Shows how the component handles API errors.',
      },
    },
  },
  beforeEach: () => {
    const mockApi = {
      getTrending: vi.fn().mockRejectedValue(new Error('Failed to load trending content')),
    };
    vi.mocked(getStreamingApi).mockReturnValue(mockApi as any);
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Should show error message
    await expect(canvas.getByText(/error|failed/i)).toBeInTheDocument();
  },
};

// Malformed data story (tests "d is not iterable" scenario)
export const MalformedData: Story = {
  parameters: {
    docs: {
      description: {
        story: 'Tests how the component handles malformed API responses that could cause iteration errors.',
      },
    },
  },
  beforeEach: () => {
    const mockApi = {
      getTrending: vi.fn().mockResolvedValue({
        success: true,
        data: null, // This could cause "d is not iterable" error
      }),
    };
    vi.mocked(getStreamingApi).mockReturnValue(mockApi as any);
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Component should handle malformed data gracefully, not crash
    // Should show error state or fallback content
    await expect(
      canvas.getByText(/error|no.*data|failed/i) || 
      canvas.getByTestId('trending-carousel')
    ).toBeInTheDocument();
  },
};