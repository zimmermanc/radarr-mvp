import React, { useEffect, useState } from 'react';
import { 
  FilmIcon, 
  PlayIcon, 
  ClockIcon, 
  CheckCircleIcon,
  ExclamationTriangleIcon 
} from '@heroicons/react/24/outline';
import { radarrApi, isApiError } from '../lib/api';
import type { Movie, HealthResponse } from '../types/api';
import { usePageTitle } from '../contexts/UIContext';
import { useToast, useApiErrorHandler } from '../components/ui/Toast';
import { DashboardStatsSkeleton, PageLoading, LoadingButton } from '../components/ui/Loading';
import ConnectionTest from '../components/ConnectionTest';

interface DashboardStats {
  totalMovies: number;
  moviesWithFiles: number;
  monitored: number;
  unmonitored: number;
}

export const Dashboard: React.FC = () => {
  usePageTitle('Dashboard');

  const [stats, setStats] = useState<DashboardStats>({
    totalMovies: 0,
    moviesWithFiles: 0,
    monitored: 0,
    unmonitored: 0,
  });
  const [recentMovies, setRecentMovies] = useState<Movie[]>([]);
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [retryLoading, setRetryLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { success } = useToast();
  const handleApiError = useApiErrorHandler();

  useEffect(() => {
    loadDashboardData();
  }, []);

  const loadDashboardData = async (isRetry = false) => {
    try {
      if (isRetry) {
        setRetryLoading(true);
      } else {
        setLoading(true);
      }
      setError(null);

      // Load health status
      const healthResponse = await radarrApi.getHealth();
      if (isApiError(healthResponse)) {
        handleApiError(healthResponse.error, 'Load health status');
        throw new Error(healthResponse.error.message);
      }
      setHealth(healthResponse.data);

      // Load movies for stats
      const moviesResponse = await radarrApi.getMovies({ limit: 20 });
      if (isApiError(moviesResponse)) {
        handleApiError(moviesResponse.error, 'Load movies');
        throw new Error(moviesResponse.error.message);
      }

      const { movies, total } = moviesResponse.data;
      
      // Calculate stats
      const moviesWithFiles = movies.filter(m => m.has_file).length;
      const monitored = movies.filter(m => m.monitored).length;
      const unmonitored = movies.filter(m => !m.monitored).length;

      setStats({
        totalMovies: total,
        moviesWithFiles,
        monitored,
        unmonitored,
      });

      // Get recent movies (last 5 added)
      setRecentMovies(movies.slice(0, 5));

      if (isRetry) {
        success('Dashboard Refreshed', 'Dashboard data has been refreshed successfully');
      }

    } catch (err) {
      console.error('Failed to load dashboard data:', err);
      setError(err instanceof Error ? err.message : 'Failed to load dashboard data');
    } finally {
      setLoading(false);
      setRetryLoading(false);
    }
  };

  const statCards = [
    {
      title: 'Total Movies',
      value: stats.totalMovies,
      icon: FilmIcon,
      color: 'text-primary-600 bg-primary-100',
      description: 'Movies in library'
    },
    {
      title: 'Downloaded',
      value: stats.moviesWithFiles,
      icon: CheckCircleIcon,
      color: 'text-success-600 bg-success-100',
      description: 'Movies with files'
    },
    {
      title: 'Monitored',
      value: stats.monitored,
      icon: PlayIcon,
      color: 'text-warning-600 bg-warning-100',
      description: 'Actively monitored'
    },
    {
      title: 'Unmonitored',
      value: stats.unmonitored,
      icon: ClockIcon,
      color: 'text-secondary-600 bg-secondary-100',
      description: 'Not monitored'
    },
  ];

  if (loading) {
    return (
      <div className="p-6 space-y-6">
        <PageLoading message="Loading dashboard data..." size="lg" />
        <DashboardStatsSkeleton />
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6">
        <div className="card p-6 border-error-200 bg-error-50 dark:bg-error-900/20">
          <div className="flex items-center">
            <ExclamationTriangleIcon className="h-6 w-6 text-error-600 mr-3" />
            <div>
              <h3 className="text-lg font-medium text-error-800 dark:text-error-200">
                Failed to Load Dashboard
              </h3>
              <p className="text-error-600 dark:text-error-300">{error}</p>
              <LoadingButton
                onClick={() => loadDashboardData(true)}
                loading={retryLoading}
                loadingText="Retrying..."
                variant="primary"
                className="mt-3"
              >
                Retry
              </LoadingButton>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-secondary-900 dark:text-white">
            Dashboard
          </h1>
          <p className="text-secondary-600 dark:text-secondary-400">
            Overview of your movie collection
          </p>
        </div>
        {health && (
          <div className="flex items-center space-x-2">
            <div className="h-3 w-3 bg-success-500 rounded-full animate-pulse"></div>
            <span className="text-sm text-secondary-600 dark:text-secondary-400">
              API Status: {health.status}
            </span>
          </div>
        )}
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {statCards.map((stat) => (
          <div key={stat.title} className="card p-6">
            <div className="flex items-center">
              <div className={`p-3 rounded-lg ${stat.color}`}>
                <stat.icon className="h-6 w-6" />
              </div>
              <div className="ml-4">
                <p className="text-sm font-medium text-secondary-600 dark:text-secondary-400">
                  {stat.title}
                </p>
                <p className="text-2xl font-semibold text-secondary-900 dark:text-white">
                  {stat.value}
                </p>
                <p className="text-xs text-secondary-500 dark:text-secondary-400">
                  {stat.description}
                </p>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Connection Test */}
      <ConnectionTest />

      {/* Recent Movies */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-secondary-900 dark:text-white mb-4">
          Recent Movies
        </h2>
        {recentMovies.length > 0 ? (
          <div className="space-y-3">
            {recentMovies.map((movie) => (
              <div
                key={movie.id}
                className="flex items-center justify-between p-3 bg-secondary-50 dark:bg-secondary-700/50 rounded-lg"
              >
                <div className="flex items-center space-x-3">
                  <div className="flex-shrink-0">
                    {movie.poster_path ? (
                      <img
                        src={`https://image.tmdb.org/t/p/w92${movie.poster_path}`}
                        alt={movie.title}
                        className="h-12 w-8 object-cover rounded"
                      />
                    ) : (
                      <div className="h-12 w-8 bg-secondary-300 dark:bg-secondary-600 rounded flex items-center justify-center">
                        <FilmIcon className="h-4 w-4 text-secondary-500" />
                      </div>
                    )}
                  </div>
                  <div>
                    <p className="font-medium text-secondary-900 dark:text-white">
                      {movie.title} ({movie.year})
                    </p>
                    <p className="text-sm text-secondary-500 dark:text-secondary-400">
                      {movie.has_file ? 'Downloaded' : 'Missing'}
                    </p>
                  </div>
                </div>
                <div className="flex items-center space-x-2">
                  {movie.has_file && (
                    <CheckCircleIcon className="h-5 w-5 text-success-500" />
                  )}
                  {movie.monitored && (
                    <PlayIcon className="h-5 w-5 text-primary-500" />
                  )}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8">
            <FilmIcon className="h-12 w-12 text-secondary-400 mx-auto mb-3" />
            <p className="text-secondary-500 dark:text-secondary-400">
              No movies found. Add some movies to get started!
            </p>
          </div>
        )}
      </div>
    </div>
  );
};