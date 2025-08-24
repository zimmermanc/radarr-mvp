import React, { useState, useEffect } from 'react';
import { 
  XMarkIcon, 
  FilmIcon, 
  ClockIcon,
  StarIcon,
  PlayIcon,
  ArrowDownTrayIcon,
  MagnifyingGlassIcon,
  TagIcon
} from '@heroicons/react/24/outline';
import { CheckCircleIcon, ExclamationCircleIcon } from '@heroicons/react/24/solid';
import type { Movie, QueueItem, SearchRelease } from '../types/api';
import { radarrApi, isApiError } from '../lib/api';
import { useToast } from '../components/ui/Toast';
import { LoadingButton } from '../components/ui/Loading';
import { MovieSearchModal } from './MovieSearchModal';

interface MovieDetailModalProps {
  movie: Movie;
  isOpen: boolean;
  onClose: () => void;
  onUpdate: (movie: Movie) => void;
}

export const MovieDetailModal: React.FC<MovieDetailModalProps> = ({
  movie,
  isOpen,
  onClose,
  onUpdate
}) => {
  const [isLoading, setIsLoading] = useState(false);
  const [downloadQueue, setDownloadQueue] = useState<QueueItem[]>([]);
  const [showSearchModal, setShowSearchModal] = useState(false);
  const [queueLoading, setQueueLoading] = useState(false);
  const { success, error: toastError } = useToast();

  useEffect(() => {
    if (isOpen) {
      // Load download queue for this movie
      loadDownloadQueue();
    }
  }, [isOpen, movie.id]); // loadDownloadQueue is stable as it doesn't depend on external variables

  const loadDownloadQueue = async () => {
    try {
      setQueueLoading(true);
      const response = await radarrApi.getQueue();
      
      if (isApiError(response)) {
        console.error('Failed to load download queue:', response.error);
        return;
      }

      // Filter queue items for this specific movie
      const movieQueueItems = response.data.items.filter(item => item.movieId === movie.id);
      setDownloadQueue(movieQueueItems);
    } catch (error) {
      console.error('Failed to load download queue:', error);
    } finally {
      setQueueLoading(false);
    }
  };

  const handleToggleMonitored = async () => {
    setIsLoading(true);
    try {
      const response = await radarrApi.updateMovie(movie.id, {
        monitored: !movie.monitored
      });

      if (isApiError(response)) {
        toastError('Failed to update movie', response.error.message);
        return;
      }

      onUpdate(response.data);
      success(
        movie.monitored ? 'Movie Unmonitored' : 'Movie Monitored',
        `${movie.title} is now ${!movie.monitored ? 'monitored' : 'unmonitored'}`
      );
    } catch (err) {
      toastError('Error', 'Failed to update movie monitoring status');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSearchNow = () => {
    setShowSearchModal(true);
  };

  const handleDownloadRelease = async (release: SearchRelease) => {
    try {
      const response = await radarrApi.downloadRelease({
        movieId: movie.id,
        releaseId: release.id,
        indexerId: release.indexerId,
        downloadUrl: release.downloadUrl || `mock://download/${release.id}`
      });

      if (isApiError(response)) {
        toastError('Download Failed', response.error.message);
        return;
      }

      success('Download Started', `Downloading: ${release.title}`);
      // Reload queue to show new download
      loadDownloadQueue();
    } catch (err) {
      console.error('Failed to start download:', err);
      toastError('Download Error', 'Failed to start download');
    }
  };

  if (!isOpen) return null;

  const getStatusBadge = () => {
    if (movie.has_file) {
      return (
        <span className="flex items-center px-3 py-1 rounded-full text-sm font-medium bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-400">
          <CheckCircleIcon className="h-4 w-4 mr-1" />
          Downloaded
        </span>
      );
    }
    if (movie.monitored) {
      return (
        <span className="flex items-center px-3 py-1 rounded-full text-sm font-medium bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-400">
          <ExclamationCircleIcon className="h-4 w-4 mr-1" />
          Monitored
        </span>
      );
    }
    return (
      <span className="flex items-center px-3 py-1 rounded-full text-sm font-medium bg-secondary-100 text-secondary-700 dark:bg-secondary-700 dark:text-secondary-300">
        Unmonitored
      </span>
    );
  };

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0">
        {/* Background overlay */}
        <div 
          className="fixed inset-0 bg-black bg-opacity-50 transition-opacity"
          onClick={onClose}
        />

        {/* Modal panel */}
        <div className="inline-block align-bottom bg-white dark:bg-secondary-800 rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-4xl sm:w-full">
          {/* Header with backdrop */}
          <div className="relative">
            {movie.backdrop_path && (
              <div className="h-64 w-full overflow-hidden">
                <img
                  src={`https://image.tmdb.org/t/p/w1280${movie.backdrop_path}`}
                  alt={movie.title}
                  className="w-full h-full object-cover"
                />
                <div className="absolute inset-0 bg-gradient-to-t from-black/70 to-transparent" />
              </div>
            )}
            
            {/* Close button */}
            <button
              onClick={onClose}
              className="absolute top-4 right-4 p-2 rounded-full bg-black/50 text-white hover:bg-black/70 transition-colors"
            >
              <XMarkIcon className="h-6 w-6" />
            </button>

            {/* Movie title and status */}
            <div className={`absolute bottom-0 left-0 right-0 p-6 ${movie.backdrop_path ? 'text-white' : 'text-secondary-900 dark:text-white'}`}>
              <div className="flex items-start justify-between">
                <div>
                  <h2 className="text-3xl font-bold mb-2">{movie.title}</h2>
                  <div className="flex items-center space-x-4 text-sm">
                    <span>{movie.year}</span>
                    {movie.runtime && (
                      <>
                        <span>•</span>
                        <span className="flex items-center">
                          <ClockIcon className="h-4 w-4 mr-1" />
                          {movie.runtime} min
                        </span>
                      </>
                    )}
                    {movie.vote_average && (
                      <>
                        <span>•</span>
                        <span className="flex items-center">
                          <StarIcon className="h-4 w-4 mr-1" />
                          {movie.vote_average.toFixed(1)}
                        </span>
                      </>
                    )}
                  </div>
                </div>
                <div className="ml-4">
                  {getStatusBadge()}
                </div>
              </div>
            </div>
          </div>

          {/* Content */}
          <div className="px-6 py-4">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              {/* Poster and actions */}
              <div className="md:col-span-1">
                {movie.poster_path ? (
                  <img
                    src={`https://image.tmdb.org/t/p/w342${movie.poster_path}`}
                    alt={movie.title}
                    className="w-full rounded-lg shadow-lg"
                  />
                ) : (
                  <div className="w-full aspect-[2/3] bg-secondary-300 dark:bg-secondary-600 rounded-lg flex items-center justify-center">
                    <FilmIcon className="h-16 w-16 text-secondary-500" />
                  </div>
                )}

                {/* Action buttons */}
                <div className="mt-4 space-y-2">
                  <LoadingButton
                    onClick={handleToggleMonitored}
                    loading={isLoading}
                    variant={movie.monitored ? 'secondary' : 'primary'}
                    className="w-full justify-center"
                  >
                    {movie.monitored ? 'Unmonitor' : 'Monitor'}
                  </LoadingButton>

                  {!movie.has_file && (
                    <button
                      onClick={handleSearchNow}
                      className="btn-primary w-full justify-center"
                    >
                      <MagnifyingGlassIcon className="h-5 w-5 mr-2" />
                      Search Now
                    </button>
                  )}

                  {movie.has_file && (
                    <button className="btn-secondary w-full justify-center">
                      <PlayIcon className="h-5 w-5 mr-2" />
                      Play Movie
                    </button>
                  )}
                </div>
              </div>

              {/* Movie details */}
              <div className="md:col-span-2 space-y-4">
                {/* Overview */}
                {movie.overview && (
                  <div>
                    <h3 className="text-lg font-semibold text-secondary-900 dark:text-white mb-2">
                      Overview
                    </h3>
                    <p className="text-secondary-600 dark:text-secondary-400">
                      {movie.overview}
                    </p>
                  </div>
                )}

                {/* Details grid */}
                <div>
                  <h3 className="text-lg font-semibold text-secondary-900 dark:text-white mb-2">
                    Details
                  </h3>
                  <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
                    {movie.release_date && (
                      <>
                        <dt className="text-secondary-500 dark:text-secondary-400">Release Date</dt>
                        <dd className="text-secondary-900 dark:text-white">
                          {new Date(movie.release_date).toLocaleDateString()}
                        </dd>
                      </>
                    )}
                    {movie.genres && movie.genres.length > 0 && (
                      <>
                        <dt className="text-secondary-500 dark:text-secondary-400">Genres</dt>
                        <dd className="text-secondary-900 dark:text-white">
                          {movie.genres.join(', ')}
                        </dd>
                      </>
                    )}
                    {movie.imdb_id && (
                      <>
                        <dt className="text-secondary-500 dark:text-secondary-400">IMDb ID</dt>
                        <dd className="text-secondary-900 dark:text-white">
                          <a 
                            href={`https://www.imdb.com/title/${movie.imdb_id}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-primary-600 hover:text-primary-700 dark:text-primary-400"
                          >
                            {movie.imdb_id}
                          </a>
                        </dd>
                      </>
                    )}
                    {movie.tmdb_id && (
                      <>
                        <dt className="text-secondary-500 dark:text-secondary-400">TMDB ID</dt>
                        <dd className="text-secondary-900 dark:text-white">{movie.tmdb_id}</dd>
                      </>
                    )}
                    {movie.file_path && (
                      <>
                        <dt className="text-secondary-500 dark:text-secondary-400">File Path</dt>
                        <dd className="text-secondary-900 dark:text-white text-xs break-all">
                          {movie.file_path}
                        </dd>
                      </>
                    )}
                  </dl>
                </div>

                {/* Tags */}
                {movie.tags && movie.tags.length > 0 && (
                  <div>
                    <h3 className="text-lg font-semibold text-secondary-900 dark:text-white mb-2">
                      Tags
                    </h3>
                    <div className="flex flex-wrap gap-2">
                      {movie.tags.map((tag, index) => (
                        <span
                          key={index}
                          className="inline-flex items-center px-3 py-1 rounded-full text-sm bg-secondary-100 text-secondary-700 dark:bg-secondary-700 dark:text-secondary-300"
                        >
                          <TagIcon className="h-4 w-4 mr-1" />
                          {tag}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {/* Download Queue */}
                {queueLoading ? (
                  <div>
                    <h3 className="text-lg font-semibold text-secondary-900 dark:text-white mb-2">
                      Download Queue
                    </h3>
                    <div className="animate-pulse space-y-2">
                      <div className="h-16 bg-secondary-200 dark:bg-secondary-700 rounded"></div>
                    </div>
                  </div>
                ) : downloadQueue.length > 0 ? (
                  <div>
                    <h3 className="text-lg font-semibold text-secondary-900 dark:text-white mb-2">
                      Download Queue ({downloadQueue.length})
                    </h3>
                    <div className="space-y-2">
                      {downloadQueue.map((item) => (
                        <div key={item.id} className="flex items-center justify-between p-3 bg-secondary-50 dark:bg-secondary-700 rounded">
                          <div className="flex items-center">
                            <ArrowDownTrayIcon className="h-5 w-5 text-primary-600 mr-3" />
                            <div>
                              <p className="text-sm font-medium text-secondary-900 dark:text-white">
                                {item.movieTitle}
                              </p>
                              <p className="text-xs text-secondary-500 dark:text-secondary-400">
                                {(item.size / (1024 * 1024 * 1024)).toFixed(2)} GB • {item.quality} • {item.indexer}
                              </p>
                              <p className="text-xs text-secondary-500 dark:text-secondary-400">
                                Status: {item.status}
                              </p>
                            </div>
                          </div>
                          <div className="text-right">
                            <p className="text-sm font-medium text-secondary-900 dark:text-white">
                              {item.progress.toFixed(1)}%
                            </p>
                            {item.timeleft && (
                              <p className="text-xs text-secondary-500 dark:text-secondary-400">
                                {item.timeleft}
                              </p>
                            )}
                            {item.errorMessage && (
                              <p className="text-xs text-error-500">
                                Error: {item.errorMessage}
                              </p>
                            )}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ) : null}
              </div>
            </div>
          </div>

          {/* Footer */}
          <div className="px-6 py-4 bg-secondary-50 dark:bg-secondary-900 flex justify-end space-x-3">
            <button
              onClick={onClose}
              className="btn-ghost"
            >
              Close
            </button>
          </div>
        </div>
      </div>

      {/* Search Modal */}
      <MovieSearchModal
        movieTitle={movie.title}
        movieId={movie.id}
        isOpen={showSearchModal}
        onClose={() => setShowSearchModal(false)}
        onDownload={handleDownloadRelease}
      />
    </div>
  );
};