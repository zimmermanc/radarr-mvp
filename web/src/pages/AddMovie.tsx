import React, { useState, useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';
import { 
  MagnifyingGlassIcon,
  PlusIcon,
  FilmIcon,
  ExclamationTriangleIcon,
  CheckCircleIcon 
} from '@heroicons/react/24/outline';
import { radarrApi, isApiError } from '../lib/api';
import type { SearchResult, QualityProfile } from '../types/api';
import { usePageTitle } from '../contexts/UIContext';
// import { useToast } from '../components/ui/Toast'; // Currently unused

export const AddMovie: React.FC = () => {
  usePageTitle('Add Movie');
  const [searchParams] = useSearchParams();
  const initialSearchTerm = searchParams.get('search') || '';

  const [searchTerm, setSearchTerm] = useState(initialSearchTerm);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [qualityProfiles, setQualityProfiles] = useState<QualityProfile[]>([]);
  const [selectedQualityProfile, setSelectedQualityProfile] = useState<number>(1);
  const [loading, setLoading] = useState(false);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [addingMovies, setAddingMovies] = useState<Set<number>>(new Set());
  const [addedMovies, setAddedMovies] = useState<Set<number>>(new Set());

  // const { success } = useToast(); // Currently unused

  useEffect(() => {
    loadQualityProfiles();
    // If there's an initial search term from URL, perform search automatically
    if (initialSearchTerm) {
      performSearch(initialSearchTerm);
    }
  }, [initialSearchTerm]);

  const loadQualityProfiles = async () => {
    try {
      setLoading(true);
      const response = await radarrApi.getQualityProfiles();
      
      if (isApiError(response)) {
        throw new Error(response.error.message);
      }
      
      setQualityProfiles(response.data);
      if (response.data.length > 0) {
        setSelectedQualityProfile(response.data[0].id);
      }
    } catch (err) {
      console.error('Failed to load quality profiles:', err);
      setError(err instanceof Error ? err.message : 'Failed to load quality profiles');
    } finally {
      setLoading(false);
    }
  };

  const performSearch = async (term: string) => {
    if (!term.trim()) return;

    try {
      setSearching(true);
      setError(null);
      
      const response = await radarrApi.searchMovies({
        term: term.trim(),
        limit: 20,
      });

      if (isApiError(response)) {
        throw new Error(response.error.message);
      }

      setSearchResults(response.data);
    } catch (err) {
      console.error('Search failed:', err);
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setSearching(false);
    }
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    performSearch(searchTerm);
  };

  const handleAddMovie = async (movie: SearchResult) => {
    try {
      setAddingMovies(prev => new Set([...prev, movie.tmdb_id]));
      setError(null);

      const response = await radarrApi.addMovie({
        tmdb_id: movie.tmdb_id,
        quality_profile_id: selectedQualityProfile,
        monitored: true,
        search_for_movie: true,
      });

      if (isApiError(response)) {
        throw new Error(response.error.message);
      }

      setAddedMovies(prev => new Set([...prev, movie.tmdb_id]));
      
      // Remove from search results after successful add
      setTimeout(() => {
        setSearchResults(prev => prev.filter(result => result.tmdb_id !== movie.tmdb_id));
      }, 2000);

    } catch (err) {
      console.error('Failed to add movie:', err);
      setError(err instanceof Error ? err.message : 'Failed to add movie');
    } finally {
      setAddingMovies(prev => {
        const newSet = new Set(prev);
        newSet.delete(movie.tmdb_id);
        return newSet;
      });
    }
  };

  const isMovieAdding = (tmdbId: number) => addingMovies.has(tmdbId);
  const isMovieAdded = (tmdbId: number) => addedMovies.has(tmdbId);

  if (loading) {
    return (
      <div className="p-6">
        <div className="animate-pulse">
          <div className="h-8 bg-secondary-200 dark:bg-secondary-700 rounded w-1/4 mb-6"></div>
          <div className="h-12 bg-secondary-200 dark:bg-secondary-700 rounded mb-4"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-secondary-900 dark:text-white">
          Add Movie
        </h1>
        <p className="text-secondary-600 dark:text-secondary-400">
          Search for movies to add to your collection
        </p>
      </div>

      {/* Search Form */}
      <div className="card p-6">
        <form onSubmit={handleSearch} className="space-y-4">
          <div className="flex space-x-4">
            <div className="flex-1 relative">
              <MagnifyingGlassIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 h-5 w-5 text-secondary-400" />
              <input
                type="text"
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                placeholder="Search for movies (e.g., 'The Matrix', 'Inception')..."
                className="form-input pl-10"
                disabled={searching}
              />
            </div>
            <button
              type="submit"
              disabled={searching || !searchTerm.trim()}
              className="btn-primary min-w-[100px] disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {searching ? (
                <div className="flex items-center">
                  <div className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full mr-2"></div>
                  Searching...
                </div>
              ) : (
                <>
                  <MagnifyingGlassIcon className="h-5 w-5 mr-2" />
                  Search
                </>
              )}
            </button>
          </div>

          {/* Quality Profile Selection */}
          {qualityProfiles.length > 0 && (
            <div className="flex items-center space-x-4">
              <label className="form-label mb-0">Quality Profile:</label>
              <select
                value={selectedQualityProfile}
                onChange={(e) => setSelectedQualityProfile(Number(e.target.value))}
                className="form-input w-auto"
              >
                {qualityProfiles.map((profile) => (
                  <option key={profile.id} value={profile.id}>
                    {profile.name}
                  </option>
                ))}
              </select>
            </div>
          )}
        </form>
      </div>

      {/* Error Message */}
      {error && (
        <div className="card p-4 border-error-200 bg-error-50 dark:bg-error-900/20">
          <div className="flex items-center">
            <ExclamationTriangleIcon className="h-5 w-5 text-error-600 mr-3" />
            <p className="text-error-800 dark:text-error-200">{error}</p>
          </div>
        </div>
      )}

      {/* Search Results */}
      {searchResults.length > 0 && (
        <div className="space-y-4">
          <h2 className="text-lg font-semibold text-secondary-900 dark:text-white">
            Search Results ({searchResults.length})
          </h2>
          
          <div className="space-y-4">
            {searchResults.map((movie) => (
              <div key={movie.tmdb_id} className="card p-4">
                <div className="flex items-center space-x-4">
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
                    <h3 className="text-lg font-semibold text-secondary-900 dark:text-white">
                      {movie.title}
                    </h3>
                    <p className="text-secondary-600 dark:text-secondary-400">
                      {movie.year} • Release: {movie.release_date ? new Date(movie.release_date).toLocaleDateString() : 'Unknown'}
                    </p>
                    {movie.vote_average && (
                      <div className="flex items-center space-x-1 text-sm text-secondary-600 dark:text-secondary-400 mt-1">
                        <span>⭐</span>
                        <span>{movie.vote_average.toFixed(1)}/10</span>
                        {movie.popularity && (
                          <span className="ml-3">📈 Popularity: {movie.popularity.toFixed(0)}</span>
                        )}
                      </div>
                    )}
                    {movie.overview && (
                      <p className="text-sm text-secondary-500 dark:text-secondary-400 mt-2 line-clamp-3">
                        {movie.overview}
                      </p>
                    )}
                  </div>

                  {/* Add Button */}
                  <div className="flex-shrink-0">
                    {isMovieAdded(movie.tmdb_id) ? (
                      <div className="flex items-center text-success-600">
                        <CheckCircleIcon className="h-5 w-5 mr-2" />
                        <span className="font-medium">Added</span>
                      </div>
                    ) : (
                      <button
                        onClick={() => handleAddMovie(movie)}
                        disabled={isMovieAdding(movie.tmdb_id)}
                        className="btn-primary min-w-[100px] disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {isMovieAdding(movie.tmdb_id) ? (
                          <div className="flex items-center">
                            <div className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full mr-2"></div>
                            Adding...
                          </div>
                        ) : (
                          <>
                            <PlusIcon className="h-5 w-5 mr-2" />
                            Add Movie
                          </>
                        )}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Empty State */}
      {!searching && searchResults.length === 0 && searchTerm && (
        <div className="card p-8 text-center">
          <FilmIcon className="h-16 w-16 text-secondary-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-secondary-900 dark:text-white mb-2">
            No movies found
          </h3>
          <p className="text-secondary-600 dark:text-secondary-400">
            Try searching with different keywords or check your spelling
          </p>
        </div>
      )}

      {/* Initial State */}
      {!searchTerm && searchResults.length === 0 && (
        <div className="card p-8 text-center">
          <MagnifyingGlassIcon className="h-16 w-16 text-secondary-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-secondary-900 dark:text-white mb-2">
            Search for Movies
          </h3>
          <p className="text-secondary-600 dark:text-secondary-400">
            Enter a movie title in the search box above to find movies to add to your collection
          </p>
        </div>
      )}
    </div>
  );
};