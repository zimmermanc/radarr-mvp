import React, { useEffect, useState } from 'react';
import { 
  FilmIcon, 
  MagnifyingGlassIcon,
  AdjustmentsHorizontalIcon,
  PlusIcon,
  ExclamationTriangleIcon,
  Squares2X2Icon,
  ListBulletIcon,
  PauseIcon,
  PlayIcon
} from '@heroicons/react/24/outline';
import { Link } from 'react-router-dom';
import { radarrApi, isApiError } from '../lib/api';
import type { Movie, MovieFilters, MovieSortField, SortDirection } from '../types/api';
import { usePageTitle } from '../contexts/UIContext';
import { useToast, useApiErrorHandler } from '../components/ui/Toast';
import { MovieCardSkeleton, LoadingButton } from '../components/ui/Loading';
import { MovieDetailModal } from '../components/MovieDetailModal';

type ViewMode = 'grid' | 'list';

export const Movies: React.FC = () => {
  usePageTitle('Movies');

  const [movies, setMovies] = useState<Movie[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchLoading, setSearchLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<MovieSortField>('title');
  const [sortDirection, setSortDirection] = useState<SortDirection>('asc');
  const [filters, setFilters] = useState<MovieFilters>({});
  const [showFilters, setShowFilters] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [selectedMovie, setSelectedMovie] = useState<Movie | null>(null);
  const [selectedMovies, setSelectedMovies] = useState<Set<number>>(new Set());
  const [showBulkActions, setShowBulkActions] = useState(false);

  const { success, error: toastError } = useToast();
  const handleApiError = useApiErrorHandler();

  useEffect(() => {
    loadMovies();
  }, [sortBy, sortDirection, filters]);

  const loadMovies = async (isSearch = false) => {
    try {
      if (isSearch) {
        setSearchLoading(true);
      } else {
        setLoading(true);
      }
      setError(null);

      const response = await radarrApi.getMovies({
        sort_by: sortBy,
        sort_direction: sortDirection,
        filters: {
          ...filters,
          search: searchTerm || undefined,
        },
      });

      if (isApiError(response)) {
        handleApiError(response.error, 'Load movies');
        throw new Error(response.error.message);
      }

      setMovies(response.data.movies);
      
      if (isSearch && searchTerm) {
        success('Search Complete', `Found ${response.data.movies.length} movies matching "${searchTerm}"`);
      }
    } catch (err) {
      console.error('Failed to load movies:', err);
      setError(err instanceof Error ? err.message : 'Failed to load movies');
    } finally {
      setLoading(false);
      setSearchLoading(false);
    }
  };

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    loadMovies(true);
  };

  // Removed toggleSort function as it's not being used in the current implementation

  const updateFilter = (key: keyof MovieFilters, value: MovieFilters[keyof MovieFilters]) => {
    setFilters(prev => ({ ...prev, [key]: value }));
  };

  const clearFilters = () => {
    setFilters({});
    setSearchTerm('');
  };

  const handleMovieClick = (movie: Movie) => {
    setSelectedMovie(movie);
  };

  const handleMovieUpdate = (updatedMovie: Movie) => {
    setMovies(prev => prev.map(m => m.id === updatedMovie.id ? updatedMovie : m));
    setSelectedMovie(updatedMovie);
  };

  const toggleMovieSelection = (movieId: number) => {
    setSelectedMovies(prev => {
      const newSet = new Set(prev);
      if (newSet.has(movieId)) {
        newSet.delete(movieId);
      } else {
        newSet.add(movieId);
      }
      setShowBulkActions(newSet.size > 0);
      return newSet;
    });
  };

  const selectAllMovies = () => {
    if (selectedMovies.size === movies.length) {
      setSelectedMovies(new Set());
      setShowBulkActions(false);
    } else {
      setSelectedMovies(new Set(movies.map(m => m.id)));
      setShowBulkActions(true);
    }
  };

  const handleBulkMonitor = async (monitored: boolean) => {
    try {
      const movieIds = Array.from(selectedMovies);
      const response = await radarrApi.bulkUpdateMovies({
        movieIds,
        updates: { monitored }
      });

      if (isApiError(response)) {
        toastError('Bulk Update Failed', response.error.message);
        return;
      }

      success(
        `Bulk Update Complete`, 
        `${selectedMovies.size} movie${selectedMovies.size !== 1 ? 's' : ''} ${monitored ? 'monitored' : 'unmonitored'}`
      );
      
      setSelectedMovies(new Set());
      setShowBulkActions(false);
      
      // Reload movies to reflect changes
      loadMovies();
    } catch (err) {
      console.error('Failed to update movies:', err);
      toastError('Error', 'Failed to update movies');
    }
  };

  const getStatusColor = (movie: Movie) => {
    if (movie.has_file) return 'text-success-600 bg-success-100';
    if (movie.monitored) return 'text-warning-600 bg-warning-100';
    return 'text-secondary-600 bg-secondary-100';
  };

  const getStatusText = (movie: Movie) => {
    if (movie.has_file) return 'Downloaded';
    if (movie.monitored) return 'Monitored';
    return 'Unmonitored';
  };

  if (loading) {
    return (
      <div className="p-6 space-y-6">
        <div className="animate-pulse">
          <div className="h-8 bg-secondary-200 dark:bg-secondary-700 rounded w-1/4 mb-6"></div>
        </div>
        <div className="space-y-4">
          {[1, 2, 3, 4, 5].map(i => (
            <MovieCardSkeleton key={i} />
          ))}
        </div>
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
                Failed to Load Movies
              </h3>
              <p className="text-error-600 dark:text-error-300">{error}</p>
              <LoadingButton
                onClick={() => loadMovies()}
                loading={loading}
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
            Movies
          </h1>
          <p className="text-secondary-600 dark:text-secondary-400">
            {movies.length} movies in your library
            {selectedMovies.size > 0 && ` • ${selectedMovies.size} selected`}
          </p>
        </div>
        <div className="flex items-center space-x-3">
          {/* View mode toggle */}
          <div className="flex rounded-lg bg-secondary-100 dark:bg-secondary-700 p-1">
            <button
              onClick={() => setViewMode('grid')}
              className={`p-2 rounded ${viewMode === 'grid' ? 'bg-white dark:bg-secondary-600 shadow' : 'hover:bg-secondary-200 dark:hover:bg-secondary-600'}`}
              title="Grid view"
            >
              <Squares2X2Icon className="h-5 w-5" />
            </button>
            <button
              onClick={() => setViewMode('list')}
              className={`p-2 rounded ${viewMode === 'list' ? 'bg-white dark:bg-secondary-600 shadow' : 'hover:bg-secondary-200 dark:hover:bg-secondary-600'}`}
              title="List view"
            >
              <ListBulletIcon className="h-5 w-5" />
            </button>
          </div>
          <Link to="/add-movie" className="btn-primary">
            <PlusIcon className="h-5 w-5 mr-2" />
            Add Movie
          </Link>
        </div>
      </div>

      {/* Bulk Actions */}
      {showBulkActions && (
        <div className="card p-4 bg-primary-50 dark:bg-primary-900/20 border-primary-200 dark:border-primary-800 animate-fade-in">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <button
                onClick={selectAllMovies}
                className="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400"
              >
                {selectedMovies.size === movies.length ? 'Deselect All' : 'Select All'}
              </button>
              <span className="text-sm text-secondary-600 dark:text-secondary-400">
                {selectedMovies.size} movie{selectedMovies.size !== 1 ? 's' : ''} selected
              </span>
            </div>
            <div className="flex items-center space-x-2">
              <button
                onClick={() => handleBulkMonitor(true)}
                className="btn-primary text-sm"
              >
                <PlayIcon className="h-4 w-4 mr-1" />
                Monitor Selected
              </button>
              <button
                onClick={() => handleBulkMonitor(false)}
                className="btn-secondary text-sm"
              >
                <PauseIcon className="h-4 w-4 mr-1" />
                Unmonitor Selected
              </button>
              <button
                onClick={() => {
                  setSelectedMovies(new Set());
                  setShowBulkActions(false);
                }}
                className="btn-ghost text-sm"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Search and Filters */}
      <div className="card p-4">
        <form onSubmit={handleSearch} className="flex items-center space-x-4">
          <div className="flex-1 relative">
            <MagnifyingGlassIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 h-5 w-5 text-secondary-400" />
            <input
              type="text"
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              placeholder="Search movies..."
              className="form-input pl-10"
            />
          </div>
          <LoadingButton
            type="submit"
            loading={searchLoading}
            loadingText="Searching..."
            variant="primary"
          >
            Search
          </LoadingButton>
          <button
            type="button"
            onClick={() => setShowFilters(!showFilters)}
            className={`btn-ghost ${showFilters ? 'bg-secondary-200' : ''}`}
          >
            <AdjustmentsHorizontalIcon className="h-5 w-5 mr-2" />
            Filters
          </button>
        </form>

        {/* Filter Panel */}
        {showFilters && (
          <div className="mt-4 pt-4 border-t border-secondary-200 dark:border-secondary-600">
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div>
                <label className="form-label">Status</label>
                <select
                  value={filters.monitored?.toString() || ''}
                  onChange={(e) => 
                    updateFilter('monitored', e.target.value === '' ? undefined : e.target.value === 'true')
                  }
                  className="form-input"
                >
                  <option value="">All</option>
                  <option value="true">Monitored</option>
                  <option value="false">Unmonitored</option>
                </select>
              </div>
              <div>
                <label className="form-label">Downloaded</label>
                <select
                  value={filters.has_file?.toString() || ''}
                  onChange={(e) => 
                    updateFilter('has_file', e.target.value === '' ? undefined : e.target.value === 'true')
                  }
                  className="form-input"
                >
                  <option value="">All</option>
                  <option value="true">Downloaded</option>
                  <option value="false">Missing</option>
                </select>
              </div>
              <div>
                <label className="form-label">Sort By</label>
                <select
                  value={sortBy}
                  onChange={(e) => setSortBy(e.target.value as MovieSortField)}
                  className="form-input"
                >
                  <option value="title">Title</option>
                  <option value="year">Year</option>
                  <option value="rating">Rating</option>
                  <option value="added_date">Date Added</option>
                  <option value="release_date">Release Date</option>
                </select>
              </div>
              <div className="flex items-end space-x-2">
                <button
                  type="button"
                  onClick={() => setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc')}
                  className="btn-secondary flex-1"
                >
                  {sortDirection === 'asc' ? '↑ Ascending' : '↓ Descending'}
                </button>
                <button
                  type="button"
                  onClick={clearFilters}
                  className="btn-ghost"
                >
                  Clear
                </button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Movies List */}
      {movies.length === 0 ? (
        <div className="card p-8 text-center">
          <FilmIcon className="h-16 w-16 text-secondary-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-secondary-900 dark:text-white mb-2">
            No movies found
          </h3>
          <p className="text-secondary-600 dark:text-secondary-400 mb-4">
            {searchTerm || Object.keys(filters).length > 0
              ? 'Try adjusting your search or filters'
              : 'Get started by adding your first movie'
            }
          </p>
          <Link to="/add-movie" className="btn-primary">
            <PlusIcon className="h-5 w-5 mr-2" />
            Add Movie
          </Link>
        </div>
      ) : viewMode === 'list' ? (
        <div className="space-y-4">
          {movies.map((movie) => (
            <div key={movie.id} className="card-interactive p-4 animate-fade-in">
              <div className="flex items-center space-x-4">
                {/* Selection checkbox */}
                <div className="flex-shrink-0">
                  <input
                    type="checkbox"
                    checked={selectedMovies.has(movie.id)}
                    onChange={() => toggleMovieSelection(movie.id)}
                    className="h-4 w-4 text-primary-600 rounded border-secondary-300 focus:ring-primary-500"
                  />
                </div>
                
                {/* Poster */}
                <div className="flex-shrink-0">
                  {movie.poster_path ? (
                    <img
                      src={`https://image.tmdb.org/t/p/w154${movie.poster_path}`}
                      alt={movie.title}
                      className="h-24 w-16 object-cover rounded"
                    />
                  ) : (
                    <div className="h-24 w-16 bg-secondary-300 dark:bg-secondary-600 rounded flex items-center justify-center">
                      <FilmIcon className="h-8 w-8 text-secondary-500" />
                    </div>
                  )}
                </div>

                {/* Movie Info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-start justify-between">
                    <div>
                      <h3 className="text-lg font-semibold text-secondary-900 dark:text-white truncate">
                        {movie.title}
                      </h3>
                      <p className="text-secondary-600 dark:text-secondary-400">
                        {movie.year} • {movie.runtime ? `${movie.runtime} min` : 'Unknown runtime'}
                      </p>
                      {movie.overview && (
                        <p className="text-sm text-secondary-500 dark:text-secondary-400 mt-2 line-clamp-2">
                          {movie.overview}
                        </p>
                      )}
                    </div>

                    {/* Status and Actions */}
                    <div className="flex items-center space-x-3 ml-4">
                      <span className={`px-3 py-1 rounded-full text-xs font-medium ${getStatusColor(movie)}`}>
                        {getStatusText(movie)}
                      </span>
                      {movie.vote_average && (
                        <div className="flex items-center space-x-1 text-sm text-secondary-600 dark:text-secondary-400">
                          <span>⭐</span>
                          <span>{movie.vote_average.toFixed(1)}</span>
                        </div>
                      )}
                      <button 
                        onClick={() => handleMovieClick(movie)}
                        className="btn-ghost text-sm"
                      >
                        Details
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : (
        /* Grid View */
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-4">
          {movies.map((movie) => (
            <div key={movie.id} className="card-interactive group relative animate-fade-in">
              {/* Selection checkbox */}
              <div className="absolute top-2 left-2 z-10">
                <input
                  type="checkbox"
                  checked={selectedMovies.has(movie.id)}
                  onChange={() => toggleMovieSelection(movie.id)}
                  className="h-4 w-4 text-primary-600 rounded border-secondary-300 focus:ring-primary-500 bg-white dark:bg-secondary-800"
                />
              </div>

              {/* Movie Card */}
              <div 
                className="cursor-pointer"
                onClick={() => handleMovieClick(movie)}
              >
                {/* Poster */}
                <div className="aspect-[2/3] overflow-hidden rounded-t-lg">
                  {movie.poster_path ? (
                    <img
                      src={`https://image.tmdb.org/t/p/w342${movie.poster_path}`}
                      alt={movie.title}
                      className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                    />
                  ) : (
                    <div className="w-full h-full bg-secondary-300 dark:bg-secondary-600 flex items-center justify-center">
                      <FilmIcon className="h-16 w-16 text-secondary-500" />
                    </div>
                  )}
                  
                  {/* Overlay on hover */}
                  <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                    <button className="btn-primary text-sm">
                      View Details
                    </button>
                  </div>
                </div>

                {/* Movie Info */}
                <div className="p-3">
                  <h3 className="font-semibold text-secondary-900 dark:text-white truncate" title={movie.title}>
                    {movie.title}
                  </h3>
                  <p className="text-sm text-secondary-600 dark:text-secondary-400">
                    {movie.year}
                  </p>
                  
                  {/* Status Badge */}
                  <div className="mt-2">
                    <span className={`inline-flex px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(movie)}`}>
                      {getStatusText(movie)}
                    </span>
                  </div>

                  {/* Rating */}
                  {movie.vote_average && (
                    <div className="mt-2 flex items-center space-x-1 text-sm text-secondary-600 dark:text-secondary-400">
                      <span>⭐</span>
                      <span>{movie.vote_average.toFixed(1)}</span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Movie Detail Modal */}
      {selectedMovie && (
        <MovieDetailModal
          movie={selectedMovie}
          isOpen={!!selectedMovie}
          onClose={() => setSelectedMovie(null)}
          onUpdate={handleMovieUpdate}
        />
      )}
    </div>
  );
};