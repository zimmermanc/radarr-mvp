import type { Meta, StoryObj } from '@storybook/react';
import { Movies } from './Movies';
import { within, expect, userEvent } from '@storybook/test';
import { TestWrapper } from '../test/TestWrapper';

const meta: Meta<typeof Movies> = {
  title: 'Pages/Movies',
  component: Movies,
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component: 'Movies library page showing movie collection with search and management features.',
      },
    },
    msw: {
      handlers: [
        // Default movies data handled by global MSW handlers
      ],
    },
  },
  decorators: [
    (Story) => (
      <TestWrapper>
        <div className="min-h-screen bg-gray-50 dark:bg-gray-900 p-4">
          <Story />
        </div>
      </TestWrapper>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof Movies>;

// Default populated state
export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Wait for movies to load from MSW mock data
    await expect(canvas.getByText(/movies/i)).toBeInTheDocument();
    
    // Test search functionality if present
    const searchInput = canvas.queryByRole('textbox', { name: /search/i });
    if (searchInput) {
      await userEvent.type(searchInput, 'test movie');
    }
  },
};

// Loading state
export const Loading: Story = {
  parameters: {
    msw: {
      handlers: [
        // Delay response to show loading state
        async (req, res, ctx) => {
          await new Promise(resolve => setTimeout(resolve, 2000));
          return res(ctx.json({
            data: [],
            totalCount: 0,
            currentPage: 1,
            totalPages: 0,
          }));
        },
      ],
    },
  },
};

// Empty state
export const EmptyLibrary: Story = {
  parameters: {
    msw: {
      handlers: [
        (req, res, ctx) => {
          return res(ctx.json({
            data: [],
            totalCount: 0,
            currentPage: 1,
            totalPages: 0,
          }));
        },
      ],
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Should show empty state message
    await expect(canvas.getByText(/no.*movies|empty|add.*movie/i)).toBeInTheDocument();
  },
};

// Error state
export const Error: Story = {
  parameters: {
    msw: {
      handlers: [
        (req, res, ctx) => {
          return res(
            ctx.status(500),
            ctx.json({ error: 'Failed to load movies' })
          );
        },
      ],
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    
    // Should show error message
    await expect(canvas.getByText(/error|failed/i)).toBeInTheDocument();
  },
};