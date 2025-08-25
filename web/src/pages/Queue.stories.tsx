import type { Meta, StoryObj } from '@storybook/react';
import { Queue } from './Queue';
import { within, expect, userEvent } from '@storybook/test';
import { BrowserRouter } from 'react-router-dom';
import { vi } from 'vitest';
import * as api from '../lib/api';

// Mock the API for Storybook
vi.mock('../lib/api');
const mockApi = vi.mocked(api);

const meta: Meta<typeof Queue> = {
  title: 'Pages/Queue',
  component: Queue,
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component: 'Queue management page showing download progress and queue operations.',
      },
    },
  },
  decorators: [
    (Story) => (
      <BrowserRouter>
        <div className="min-h-screen bg-gray-50 dark:bg-gray-900 p-4">
          <Story />
        </div>
      </BrowserRouter>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof Queue>;

const mockQueueItems = [
  {
    id: '1',
    movieId: 'movie-1',
    movieTitle: 'F1 (2025)',
    status: 'downloading',
    progress: 75,
    size: 4 * 1024 * 1024 * 1024, // 4GB
    indexer: 'HDBits',
    downloadClient: 'qBittorrent',
    added: '2025-08-24T20:00:00Z',
    estimatedCompletionTime: '2025-08-24T20:30:00Z',
  },
  {
    id: '2',
    movieId: 'movie-2',
    movieTitle: 'Mission: Impossible - The Final Reckoning',
    status: 'paused',
    progress: 45,
    size: 6 * 1024 * 1024 * 1024, // 6GB
    indexer: 'HDBits',
    downloadClient: 'qBittorrent',
    added: '2025-08-24T19:30:00Z',
    estimatedCompletionTime: null,
  },
  {
    id: '3',
    movieId: 'movie-3',
    movieTitle: 'Superman (2025)',
    status: 'completed',
    progress: 100,
    size: 3.5 * 1024 * 1024 * 1024, // 3.5GB
    indexer: 'HDBits',
    downloadClient: 'qBittorrent',
    added: '2025-08-24T18:00:00Z',
    estimatedCompletionTime: '2025-08-24T19:00:00Z',
  },
];

// Loading state
export const Loading: Story = {
  beforeEach: () => {
    mockApi.getQueue.mockImplementation(() => 
      new Promise(resolve => setTimeout(() => resolve({
        records: mockQueueItems,
        totalCount: 3,
      }), 2000))
    );
  },
};

// Populated queue
export const WithItems: Story = {
  beforeEach: () => {
    mockApi.getQueue.mockResolvedValue({
      records: mockQueueItems,
      totalCount: 3,
    });
    mockApi.pauseQueueItem.mockResolvedValue(undefined);
    mockApi.resumeQueueItem.mockResolvedValue(undefined);
    mockApi.removeQueueItem.mockResolvedValue(undefined);
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Wait for queue items to load
    await expect(canvas.getByText('F1 (2025)')).toBeInTheDocument();
    await expect(canvas.getByText('Mission: Impossible - The Final Reckoning')).toBeInTheDocument();
    
    // Test queue management actions
    const pauseButton = canvas.queryByRole('button', { name: /pause/i });
    if (pauseButton) {
      await userEvent.click(pauseButton);
    }
  },
};

// Empty queue state
export const Empty: Story = {
  beforeEach: () => {
    mockApi.getQueue.mockResolvedValue({
      records: [],
      totalCount: 0,
    });
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Should show empty state message
    await expect(canvas.getByText(/empty|no.*items/i)).toBeInTheDocument();
  },
};

// Error state
export const Error: Story = {
  beforeEach: () => {
    mockApi.getQueue.mockRejectedValue(new Error('Failed to load queue'));
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Should show error message
    await expect(canvas.getByText(/error|failed/i)).toBeInTheDocument();
  },
};

// Malformed data state (tests data safety)
export const MalformedData: Story = {
  parameters: {
    docs: {
      description: {
        story: 'Tests how the queue handles malformed API responses that could cause JavaScript errors.',
      },
    },
  },
  beforeEach: () => {
    // Return malformed data that could cause "d is not iterable" errors
    mockApi.getQueue.mockResolvedValue({
      records: null as any, // This could cause iteration errors
      totalCount: 'invalid' as any,
    });
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Component should handle malformed data gracefully
    await expect(
      canvas.getByText(/error|empty|queue/i) || canvas.getByRole('main')
    ).toBeInTheDocument();
  },
};