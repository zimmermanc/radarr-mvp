import React, { useState, useEffect } from 'react';
import { Calendar, Clock, Play, ChevronRight } from 'lucide-react';
import { getStreamingApi } from '../../lib/streamingApi';
import type { ComingSoon, MediaType } from '../../types/streaming';

interface ComingSoonListProps {
  mediaType: MediaType;
  region?: string;
  limit?: number;
  onMovieSelect?: (tmdbId: number) => void;
}

export const ComingSoonList: React.FC<ComingSoonListProps> = ({
  mediaType,
  region = 'US',
  limit = 10,
  onMovieSelect,
}) => {
  const [releases, setReleases] = useState<ComingSoon[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);

  useEffect(() => {
    fetchComingSoon();
  }, [mediaType, region]);

  const fetchComingSoon = async () => {
    try {
      setLoading(true);
      setError(null);
      const api = getStreamingApi();
      const response = await api.getComingSoon(mediaType, region);
      // Defensive programming: ensure entries is always an array (changed from 'releases' to 'entries')
      const entries = response?.entries || response?.data?.entries || [];
      setReleases(Array.isArray(entries) ? entries : []);
    } catch (err) {
      console.error('Failed to fetch coming soon:', err);
      setError('Failed to load upcoming releases');
    } finally {
      setLoading(false);
    }
  };

  const getImageUrl = (path?: string) => {
    if (!path) return '/placeholder-poster.jpg';
    return `https://image.tmdb.org/t/p/w185${path}`;
  };

  const getDaysUntilRelease = (releaseDate: string) => {
    const today = new Date();
    const release = new Date(releaseDate);
    const diffTime = release.getTime() - today.getTime();
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));
    return diffDays;
  };

  const formatReleaseDate = (date: string) => {
    const releaseDate = new Date(date);
    const today = new Date();
    const daysUntil = getDaysUntilRelease(date);

    if (daysUntil === 0) return 'Today';
    if (daysUntil === 1) return 'Tomorrow';
    if (daysUntil < 7) return `In ${daysUntil} days`;
    if (daysUntil < 30) return `In ${Math.ceil(daysUntil / 7)} weeks`;
    
    return releaseDate.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: releaseDate.getFullYear() !== today.getFullYear() ? 'numeric' : undefined,
    });
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

  // Defensive programming: ensure safe array operations
  const safeReleases = Array.isArray(releases) ? releases : [];
  const displayedReleases = showAll ? safeReleases : safeReleases.slice(0, limit);

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
      <div className="flex items-center gap-3 mb-6">
        <Calendar className="w-6 h-6 text-purple-500" />
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
          Coming Soon to Streaming
        </h2>
      </div>

      <div className="space-y-3">
        {displayedReleases.map((release) => {
          const daysUntil = getDaysUntilRelease(release.release_date);
          const isVeryS = daysUntil <= 7;
          
          return (
            <div
              key={`${release.tmdb_id}-${release.release_date}`}
              className="flex gap-4 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors cursor-pointer"
              onClick={() => onMovieSelect?.(release.tmdb_id)}
            >
              <img
                src={getImageUrl(release.poster_path)}
                alt={release.title}
                className="w-16 h-24 object-cover rounded"
              />
              
              <div className="flex-1">
                <div className="flex items-start justify-between">
                  <div>
                    <h3 className="font-semibold text-gray-900 dark:text-white">
                      {release.title}
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400 line-clamp-2 mt-1">
                      {release.overview}
                    </p>
                  </div>
                  
                  <div className={`flex items-center gap-1 px-3 py-1 rounded-full text-sm font-medium ${
                    isVeryS
                      ? 'bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400'
                      : 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400'
                  }`}>
                    <Clock className="w-4 h-4" />
                    <span>{formatReleaseDate(release.release_date)}</span>
                  </div>
                </div>

                {release.streaming_services.length > 0 && (
                  <div className="flex items-center gap-2 mt-2">
                    <Play className="w-4 h-4 text-gray-500" />
                    <div className="flex flex-wrap gap-2">
                      {release.streaming_services.map((service) => (
                        <span
                          key={service}
                          className="text-xs px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded"
                        >
                          {service}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {releases.length > limit && (
        <button
          onClick={() => setShowAll(!showAll)}
          className="w-full mt-4 flex items-center justify-center gap-2 px-4 py-2 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition-colors"
        >
          {showAll ? 'Show Less' : `Show All (${releases.length})`}
          <ChevronRight className={`w-4 h-4 transition-transform ${showAll ? 'rotate-90' : ''}`} />
        </button>
      )}
    </div>
  );
};