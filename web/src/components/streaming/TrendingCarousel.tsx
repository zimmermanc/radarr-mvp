import React, { useState, useEffect } from 'react';
import { ChevronLeft, ChevronRight, TrendingUp, Star, Calendar } from 'lucide-react';
import { getStreamingApi } from '../../lib/streamingApi';
import type { TrendingEntry, MediaType, TimeWindow } from '../../types/streaming';

interface TrendingCarouselProps {
  mediaType: MediaType;
  timeWindow?: TimeWindow;
  onMovieSelect?: (tmdbId: number) => void;
}

export const TrendingCarousel: React.FC<TrendingCarouselProps> = ({
  mediaType,
  timeWindow = 'day',
  onMovieSelect,
}) => {
  const [entries, setEntries] = useState<TrendingEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [selectedWindow, setSelectedWindow] = useState<TimeWindow>(timeWindow);

  useEffect(() => {
    fetchTrending();
  }, [mediaType, selectedWindow]);

  const fetchTrending = async () => {
    try {
      setLoading(true);
      setError(null);
      const api = getStreamingApi();
      const response = await api.getTrending(mediaType, {
        window: selectedWindow,
        limit: 20,
      });
      // Defensive programming: ensure we have a valid array
      // Fix for production "u is not iterable" error - comprehensive null safety
      const entries = response?.data?.entries || response?.entries || [];
      setEntries(Array.isArray(entries) ? entries : []);
      console.log('TrendingCarousel: Loaded', entries.length, 'entries - v1.0.3-fixed');
    } catch (err) {
      console.error('Failed to fetch trending:', err);
      setError('Failed to load trending content');
    } finally {
      setLoading(false);
    }
  };

  const handlePrevious = () => {
    setCurrentIndex((prev) => Math.max(0, prev - 1));
  };

  const handleNext = () => {
    setCurrentIndex((prev) => Math.min(entries.length - 5, prev + 1));
  };

  const handleWindowChange = (window: TimeWindow) => {
    setSelectedWindow(window);
    setCurrentIndex(0);
  };

  const getImageUrl = (path?: string) => {
    if (!path) return '/placeholder-poster.jpg';
    return `https://image.tmdb.org/t/p/w342${path}`;
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 dark:bg-red-900/20 p-4 rounded-lg">
        <p className="text-red-600 dark:text-red-400">{error}</p>
      </div>
    );
  }

  const visibleEntries = Array.isArray(entries) ? entries.slice(currentIndex, currentIndex + 5) : [];

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <TrendingUp className="w-6 h-6 text-blue-500" />
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Trending {mediaType === 'movie' ? 'Movies' : 'TV Shows'}
          </h2>
        </div>
        
        <div className="flex gap-2">
          <button
            onClick={() => handleWindowChange('day')}
            className={`px-4 py-2 rounded-lg transition-colors ${
              selectedWindow === 'day'
                ? 'bg-blue-500 text-white'
                : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'
            }`}
          >
            Today
          </button>
          <button
            onClick={() => handleWindowChange('week')}
            className={`px-4 py-2 rounded-lg transition-colors ${
              selectedWindow === 'week'
                ? 'bg-blue-500 text-white'
                : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'
            }`}
          >
            This Week
          </button>
        </div>
      </div>

      <div className="relative">
        <div className="flex gap-4 overflow-hidden">
          {visibleEntries.map((entry, index) => (
            <div
              key={`${entry.tmdb_id}-${index}`}
              className="flex-shrink-0 w-1/5 cursor-pointer group"
              onClick={() => onMovieSelect?.(entry.tmdb_id)}
            >
              <div className="relative aspect-[2/3] rounded-lg overflow-hidden">
                <img
                  src={getImageUrl(entry.poster_path)}
                  alt={entry.title}
                  className="w-full h-full object-cover transition-transform group-hover:scale-105"
                />
                {entry.rank && (
                  <div className="absolute top-2 left-2 bg-black/75 text-white px-2 py-1 rounded text-sm font-bold">
                    #{entry.rank}
                  </div>
                )}
                <div className="absolute inset-0 bg-gradient-to-t from-black/75 via-transparent opacity-0 group-hover:opacity-100 transition-opacity">
                  <div className="absolute bottom-0 p-3 text-white">
                    <p className="font-semibold line-clamp-2">{entry.title}</p>
                    <div className="flex items-center gap-2 mt-1 text-sm">
                      {entry.vote_average && (
                        <div className="flex items-center gap-1">
                          <Star className="w-4 h-4 fill-yellow-400 text-yellow-400" />
                          <span>{entry.vote_average.toFixed(1)}</span>
                        </div>
                      )}
                      {entry.release_date && (
                        <div className="flex items-center gap-1">
                          <Calendar className="w-4 h-4" />
                          <span>{new Date(entry.release_date).getFullYear()}</span>
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>

        {currentIndex > 0 && (
          <button
            onClick={handlePrevious}
            className="absolute left-0 top-1/2 -translate-y-1/2 -translate-x-4 bg-white dark:bg-gray-800 shadow-lg rounded-full p-2 hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            <ChevronLeft className="w-6 h-6" />
          </button>
        )}

        {currentIndex < entries.length - 5 && (
          <button
            onClick={handleNext}
            className="absolute right-0 top-1/2 -translate-y-1/2 translate-x-4 bg-white dark:bg-gray-800 shadow-lg rounded-full p-2 hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            <ChevronRight className="w-6 h-6" />
          </button>
        )}
      </div>

      <div className="flex justify-center mt-4 gap-1">
        {Array.from({ length: Math.ceil(Math.max(entries.length, 0) / 5) || 0 }).map((_, i) => (
          <button
            key={i}
            onClick={() => setCurrentIndex(i * 5)}
            className={`w-2 h-2 rounded-full transition-colors ${
              Math.floor(currentIndex / 5) === i
                ? 'bg-blue-500'
                : 'bg-gray-300 dark:bg-gray-600'
            }`}
          />
        ))}
      </div>
    </div>
  );
};