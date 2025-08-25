import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MovieCardWithStreaming } from './MovieCardWithStreaming';
import type { TrendingEntry } from '../../lib/schemas';

const mockMovie: TrendingEntry = {
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
};

describe('MovieCardWithStreaming', () => {
  it('should render movie information correctly', () => {
    const onAddToQueue = vi.fn();
    
    render(
      <MovieCardWithStreaming 
        movie={mockMovie} 
        onAddToQueue={onAddToQueue}
      />
    );

    // Should display movie title
    expect(screen.getByText('Test Movie')).toBeInTheDocument();
    
    // Should display vote average
    expect(screen.getByText('8.5')).toBeInTheDocument();
    
    // Should display year from release date
    expect(screen.getByText('2025')).toBeInTheDocument();
  });

  it('should handle missing poster gracefully', () => {
    const movieWithoutPoster = {
      ...mockMovie,
      poster_path: null,
    };
    
    const onAddToQueue = vi.fn();
    
    render(
      <MovieCardWithStreaming 
        movie={movieWithoutPoster} 
        onAddToQueue={onAddToQueue}
      />
    );

    // Should render without crashing
    expect(screen.getByText('Test Movie')).toBeInTheDocument();
    
    // Should show placeholder or fallback image
    const images = screen.getAllByRole('img');
    expect(images.length).toBeGreaterThan(0);
  });

  it('should call onAddToQueue when add button is clicked', () => {
    const onAddToQueue = vi.fn();
    
    render(
      <MovieCardWithStreaming 
        movie={mockMovie} 
        onAddToQueue={onAddToQueue}
      />
    );

    // Find and click add button
    const addButton = screen.getByRole('button', { name: /add|download|\+/i });
    fireEvent.click(addButton);

    // Should call callback with movie data
    expect(onAddToQueue).toHaveBeenCalledWith(mockMovie);
  });

  it('should handle undefined movie data gracefully', () => {
    const onAddToQueue = vi.fn();
    
    // Test with undefined movie (could cause crashes)
    expect(() => {
      render(
        <MovieCardWithStreaming 
          movie={undefined as any} 
          onAddToQueue={onAddToQueue}
        />
      );
    }).not.toThrow();
  });

  it('should handle malformed movie data', () => {
    const malformedMovie = {
      // Missing required fields that could cause errors
      tmdb_id: null,
      title: '',
      vote_average: 'invalid' as any,
    };
    
    const onAddToQueue = vi.fn();
    
    // Should not crash with malformed data
    expect(() => {
      render(
        <MovieCardWithStreaming 
          movie={malformedMovie as any} 
          onAddToQueue={onAddToQueue}
        />
      );
    }).not.toThrow();
  });
});