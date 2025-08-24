import React, { useState } from 'react';
import { TrendingUp, Tv, Film, RefreshCw } from 'lucide-react';
import {
  TrendingCarousel,
  ComingSoonList,
  ProviderFilter,
  StreamingAvailability,
} from '../components/streaming';
import { getStreamingApi } from '../lib/streamingApi';
import type { MediaType } from '../types/streaming';

const Streaming: React.FC = () => {
  const [mediaType, setMediaType] = useState<MediaType>('movie');
  const [selectedProviders, setSelectedProviders] = useState<string[]>([]);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [selectedTmdbId, setSelectedTmdbId] = useState<number | null>(null);

  // Streaming API is now initialized globally in App.tsx

  const handleMovieSelect = (tmdbId: number) => {
    setSelectedTmdbId(tmdbId);
    // You could navigate to movie details or open a modal here
    // navigate(`/movie/${tmdbId}`);
  };

  const handleRefreshCache = async () => {
    try {
      setIsRefreshing(true);
      const api = getStreamingApi();
      await api.refreshCache();
      // Trigger re-render of components
      window.location.reload();
    } catch (error) {
      console.error('Failed to refresh cache:', error);
    } finally {
      setIsRefreshing(false);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <div className="bg-white dark:bg-gray-800 shadow-sm border-b border-gray-200 dark:border-gray-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <TrendingUp className="w-8 h-8 text-blue-500" />
              <div>
                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
                  Streaming Discovery
                </h1>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  Discover trending content and streaming availability
                </p>
              </div>
            </div>

            <div className="flex items-center gap-4">
              {/* Media Type Toggle */}
              <div className="flex bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
                <button
                  onClick={() => setMediaType('movie')}
                  className={`flex items-center gap-2 px-4 py-2 rounded-md transition-colors ${
                    mediaType === 'movie'
                      ? 'bg-white dark:bg-gray-600 text-blue-600 dark:text-blue-400 shadow-sm'
                      : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
                  }`}
                >
                  <Film className="w-4 h-4" />
                  Movies
                </button>
                <button
                  onClick={() => setMediaType('tv')}
                  className={`flex items-center gap-2 px-4 py-2 rounded-md transition-colors ${
                    mediaType === 'tv'
                      ? 'bg-white dark:bg-gray-600 text-blue-600 dark:text-blue-400 shadow-sm'
                      : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
                  }`}
                >
                  <Tv className="w-4 h-4" />
                  TV Shows
                </button>
              </div>

              {/* Refresh Cache Button */}
              <button
                onClick={handleRefreshCache}
                disabled={isRefreshing}
                className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                <RefreshCw className={`w-4 h-4 ${isRefreshing ? 'animate-spin' : ''}`} />
                Refresh
              </button>
            </div>
          </div>

          {/* Provider Filter */}
          <div className="mt-4">
            <ProviderFilter
              selectedProviders={selectedProviders}
              onProvidersChange={setSelectedProviders}
            />
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
        {/* Trending Section */}
        <section>
          <TrendingCarousel
            mediaType={mediaType}
            onMovieSelect={handleMovieSelect}
          />
        </section>

        {/* Selected Movie Availability */}
        {selectedTmdbId && (
          <section className="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
            <StreamingAvailability tmdbId={selectedTmdbId} />
          </section>
        )}

        {/* Coming Soon Section */}
        <section>
          <ComingSoonList
            mediaType={mediaType}
            onMovieSelect={handleMovieSelect}
          />
        </section>
      </div>
    </div>
  );
};

export default Streaming;