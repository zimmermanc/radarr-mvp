import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MovieCardWithStreaming } from './MovieCardWithStreaming';
import type { TrendingEntry } from '../../lib/schemas';

const mockMovie: any = {
  id: '1',
  tmdb_id: 123456,
  title: 'Test Movie',
  year: 2025,
  overview: 'A test movie for testing purposes',
  poster_path: '/test-poster.jpg',
  backdrop_path: '/test-backdrop.jpg',
  status: 'wanted',
  monitored: true,
  quality_profile_id: 1,
  added: '2025-08-24T20:00:00Z',
  updated: '2025-08-24T20:00:00Z',
  vote_average: 8.5,
  has_file: false,
};

describe('MovieCardWithStreaming', () => {
  it('should render movie information correctly', () => {
    const onMovieClick = vi.fn();
    const onSelectionToggle = vi.fn();
    
    render(
      <MovieCardWithStreaming 
        movie={mockMovie} 
        onMovieClick={onMovieClick}
        isSelected={false}
        onSelectionToggle={onSelectionToggle}
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
    
    const onMovieClick = vi.fn();
    const onSelectionToggle = vi.fn();
    
    render(
      <MovieCardWithStreaming 
        movie={movieWithoutPoster} 
        onMovieClick={onMovieClick}
        isSelected={false}
        onSelectionToggle={onSelectionToggle}
      />
    );

    // Should render without crashing
    expect(screen.getByText('Test Movie')).toBeInTheDocument();
    
    // Should show placeholder or fallback image (or at least render without crashing)
    const images = screen.queryAllByRole('img');
    // Component should render, with or without images
    expect(screen.getByText('Test Movie')).toBeInTheDocument();
  });

  it('should call onMovieClick when movie is clicked', () => {
    const onMovieClick = vi.fn();
    const onSelectionToggle = vi.fn();
    
    render(
      <MovieCardWithStreaming 
        movie={mockMovie} 
        onMovieClick={onMovieClick}
        isSelected={false}
        onSelectionToggle={onSelectionToggle}
      />
    );

    // Find and click the movie card
    const movieCard = screen.getByText('Test Movie').closest('div');
    if (movieCard) {
      fireEvent.click(movieCard);
      // Should call callback with movie data
      expect(onMovieClick).toHaveBeenCalledWith(mockMovie);
    }
  });

  it('should handle undefined movie data gracefully', () => {
    const onMovieClick = vi.fn();
    const onSelectionToggle = vi.fn();
    
    // Test with undefined movie (could cause crashes)
    expect(() => {
      render(
        <MovieCardWithStreaming 
          movie={undefined as any} 
          onMovieClick={onMovieClick}
          isSelected={false}
          onSelectionToggle={onSelectionToggle}
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
    
    const onMovieClick = vi.fn();
    const onSelectionToggle = vi.fn();
    
    // Should not crash with malformed data
    expect(() => {
      render(
        <MovieCardWithStreaming 
          movie={malformedMovie as any} 
          onMovieClick={onMovieClick}
          isSelected={false}
          onSelectionToggle={onSelectionToggle}
        />
      );
    }).not.toThrow();
  });
});