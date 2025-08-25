import React, { useEffect, useState } from 'react';
import { Play, Film } from 'lucide-react';
import { getStreamingApi } from '../../lib/streamingApi';
import type { Movie } from '../../types/api';
import type { AvailabilityItem } from '../../types/streaming';

interface MovieCardWithStreamingProps {
  movie: Movie;
  onMovieClick: (movie: Movie) => void;
  isSelected: boolean;
  onSelectionToggle: (movieId: number) => void;
}

export const MovieCardWithStreaming: React.FC<MovieCardWithStreamingProps> = ({
  movie,
  onMovieClick,
  isSelected,
  onSelectionToggle,
}) => {
  // Handle undefined/null movie data gracefully
  if (!movie || typeof movie !== 'object') {
    return (
      <div className="card-interactive bg-gray-100 dark:bg-gray-800 p-4 rounded-lg">
        <p className="text-gray-500 dark:text-gray-400">Invalid movie data</p>
      </div>
    );
  }
  const [availability, setAvailability] = useState<AvailabilityItem[]>([]);
  const [loadingAvailability, setLoadingAvailability] = useState(false);

  useEffect(() => {
    if (movie.tmdb_id) {
      fetchAvailability();
    }
  }, [movie.tmdb_id]);

  const fetchAvailability = async () => {
    try {
      setLoadingAvailability(true);
      const api = getStreamingApi();
      const response = await api.getAvailability(movie.tmdb_id);
      setAvailability(response.items);
    } catch (err) {
      // Silently fail - streaming availability is optional
      console.error('Failed to fetch availability for movie:', movie.tmdb_id);
    } finally {
      setLoadingAvailability(false);
    }
  };

  const getStatusColor = (movie: Movie) => {
    if (movie.has_file) {
      return 'bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400';
    }
    if (movie.monitored) {
      return 'bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400';
    }
    return 'bg-secondary-100 dark:bg-secondary-700 text-secondary-600 dark:text-secondary-400';
  };

  const getStatusText = (movie: Movie) => {
    if (movie.has_file) return 'Downloaded';
    if (movie.monitored) return 'Monitored';
    return 'Not Monitored';
  };

  // Get unique streaming services (limit to 3 for display)
  const uniqueServices = Array.from(
    new Set(availability.map((item) => item.service_name))
  ).slice(0, 3);

  return (
    <div className="card-interactive group relative animate-fade-in">
      {/* Selection checkbox */}
      <div className="absolute top-2 left-2 z-10">
        <input
          type="checkbox"
          checked={isSelected}
          onChange={() => onSelectionToggle(movie.id)}
          onClick={(e) => e.stopPropagation()}
          className="h-4 w-4 text-primary-600 rounded border-secondary-300 focus:ring-primary-500 bg-white dark:bg-secondary-800"
        />
      </div>

      {/* Movie Card */}
      <div className="cursor-pointer" onClick={() => onMovieClick(movie)}>
        {/* Poster */}
        <div className="aspect-[2/3] overflow-hidden rounded-t-lg relative">
          {movie.poster_path ? (
            <img
              src={`https://image.tmdb.org/t/p/w342${movie.poster_path}`}
              alt={movie.title}
              className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
            />
          ) : (
            <div className="w-full h-full bg-secondary-300 dark:bg-secondary-600 flex items-center justify-center">
              <Film className="h-16 w-16 text-secondary-500" />
            </div>
          )}

          {/* Streaming availability badges */}
          {!loadingAvailability && uniqueServices.length > 0 && (
            <div className="absolute bottom-2 left-2 right-2">
              <div className="flex flex-wrap gap-1">
                {uniqueServices.map((service) => (
                  <span
                    key={service}
                    className="inline-flex items-center gap-1 px-2 py-1 bg-black/75 backdrop-blur-sm text-white text-xs rounded"
                    title={`Available on ${service}`}
                  >
                    <Play className="w-3 h-3" />
                    {service}
                  </span>
                ))}
                {availability.length > 3 && (
                  <span className="inline-flex items-center px-2 py-1 bg-black/75 backdrop-blur-sm text-white text-xs rounded">
                    +{availability.length - 3}
                  </span>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Movie Info */}
        <div className="p-3">
          <h3 className="font-medium text-secondary-900 dark:text-white line-clamp-1" title={movie.title}>
            {movie.title}
          </h3>
          <p className="text-sm text-secondary-600 dark:text-secondary-400 mt-1">
            {movie.year}
          </p>
          <div className="flex items-center justify-between mt-2">
            <span className={`px-2 py-0.5 rounded text-xs font-medium ${getStatusColor(movie)}`}>
              {getStatusText(movie)}
            </span>
            {movie.vote_average && (
              <div className="flex items-center space-x-1 text-xs text-secondary-600 dark:text-secondary-400">
                <span>⭐</span>
                <span>{(typeof movie.vote_average === 'number' ? movie.vote_average : 0).toFixed(1)}</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

