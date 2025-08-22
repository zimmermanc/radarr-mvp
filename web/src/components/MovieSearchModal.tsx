import React, { useState } from 'react';
import {
  XMarkIcon,
  MagnifyingGlassIcon,
  FilmIcon,
  ArrowDownTrayIcon,
  DocumentMagnifyingGlassIcon,
  ServerStackIcon,
  ClockIcon,
  TagIcon,
  UserGroupIcon
} from '@heroicons/react/24/outline';
import { useToast } from './ui/Toast';
import { LoadingButton } from './ui/Loading';

interface SearchRelease {
  id: string;
  title: string;
  indexer: string;
  indexerId: string;
  size: number;
  quality: string;
  resolution?: string;
  source?: string;
  codec?: string;
  seeders: number;
  leechers: number;
  sceneGroup?: string;
  releaseGroup?: string;
  languages?: string[];
  publishDate: string;
  downloadUrl?: string;
  infoUrl?: string;
  score?: number;
  matchType?: 'exact' | 'partial' | 'fuzzy';
}

interface MovieSearchModalProps {
  movieTitle: string;
  movieId: number;
  isOpen: boolean;
  onClose: () => void;
  onDownload: (release: SearchRelease) => void;
}

export const MovieSearchModal: React.FC<MovieSearchModalProps> = ({
  movieTitle,
  // movieId, // Reserved for future use
  isOpen,
  onClose,
  onDownload
}) => {
  const [searchResults, setSearchResults] = useState<SearchRelease[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedIndexers, setSelectedIndexers] = useState<Set<string>>(new Set(['hdbits']));
  const [filterQuality, setFilterQuality] = useState<string>('all');
  const [sortBy, setSortBy] = useState<'score' | 'size' | 'seeders' | 'date'>('score');
  const [downloadingReleases, setDownloadingReleases] = useState<Set<string>>(new Set());
  
  const { success, error: toastError } = useToast();

  const mockSearchResults: SearchRelease[] = [
    {
      id: '1',
      title: `${movieTitle}.2024.1080p.BluRay.x264-SPARKS`,
      indexer: 'HDBits',
      indexerId: 'hdb_12345',
      size: 10737418240, // 10GB
      quality: '1080p BluRay',
      resolution: '1080p',
      source: 'BluRay',
      codec: 'x264',
      seeders: 45,
      leechers: 2,
      sceneGroup: 'SPARKS',
      releaseGroup: 'SPARKS',
      languages: ['English', 'Spanish'],
      publishDate: new Date(Date.now() - 86400000).toISOString(),
      score: 95,
      matchType: 'exact'
    },
    {
      id: '2',
      title: `${movieTitle}.2024.2160p.WEB-DL.DDP5.1.Atmos.H.265-FLUX`,
      indexer: 'HDBits',
      indexerId: 'hdb_12346',
      size: 21474836480, // 20GB
      quality: '2160p WEB-DL',
      resolution: '2160p',
      source: 'WEB-DL',
      codec: 'H.265',
      seeders: 32,
      leechers: 5,
      sceneGroup: 'FLUX',
      releaseGroup: 'FLUX',
      languages: ['English'],
      publishDate: new Date(Date.now() - 172800000).toISOString(),
      score: 90,
      matchType: 'exact'
    },
    {
      id: '3',
      title: `${movieTitle}.2024.720p.WEBRip.x264-YTS`,
      indexer: 'HDBits',
      indexerId: 'hdb_12347',
      size: 1073741824, // 1GB
      quality: '720p WEBRip',
      resolution: '720p',
      source: 'WEBRip',
      codec: 'x264',
      seeders: 120,
      leechers: 15,
      releaseGroup: 'YTS',
      languages: ['English'],
      publishDate: new Date(Date.now() - 259200000).toISOString(),
      score: 75,
      matchType: 'partial'
    }
  ];

  const handleSearch = async () => {
    setIsSearching(true);
    try {
      // Simulate API call delay
      await new Promise(resolve => setTimeout(resolve, 2000));
      
      // TODO: Replace with actual API call
      // const response = await radarrApi.searchMovieReleases(movieId, { indexers: Array.from(selectedIndexers) });
      
      setSearchResults(mockSearchResults);
      success('Search Complete', `Found ${mockSearchResults.length} releases for ${movieTitle}`);
    } catch (err) {
      toastError('Search Failed', 'Unable to search for releases');
      setSearchResults([]);
    } finally {
      setIsSearching(false);
    }
  };

  const handleDownload = async (release: SearchRelease) => {
    setDownloadingReleases(prev => new Set(prev).add(release.id));
    try {
      // TODO: Implement actual download API call
      await new Promise(resolve => setTimeout(resolve, 1500));
      
      onDownload(release);
      success('Download Started', `Downloading: ${release.title}`);
      
      // Close modal after successful download
      setTimeout(() => onClose(), 1000);
    } catch (err) {
      toastError('Download Failed', 'Unable to start download');
    } finally {
      setDownloadingReleases(prev => {
        const newSet = new Set(prev);
        newSet.delete(release.id);
        return newSet;
      });
    }
  };

  const formatSize = (bytes: number): string => {
    const gb = bytes / (1024 * 1024 * 1024);
    if (gb >= 1) return `${gb.toFixed(2)} GB`;
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(0)} MB`;
  };

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    
    if (diffHours < 1) return 'Just now';
    if (diffHours < 24) return `${diffHours}h ago`;
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  const getQualityColor = (quality: string): string => {
    if (quality.includes('2160p')) return 'text-purple-600 bg-purple-100 dark:bg-purple-900/30 dark:text-purple-400';
    if (quality.includes('1080p')) return 'text-blue-600 bg-blue-100 dark:bg-blue-900/30 dark:text-blue-400';
    if (quality.includes('720p')) return 'text-green-600 bg-green-100 dark:bg-green-900/30 dark:text-green-400';
    return 'text-secondary-600 bg-secondary-100 dark:bg-secondary-700 dark:text-secondary-300';
  };

  const getScoreColor = (score: number): string => {
    if (score >= 90) return 'text-success-600';
    if (score >= 70) return 'text-warning-600';
    return 'text-error-600';
  };

  const filteredResults = searchResults
    .filter(r => filterQuality === 'all' || r.quality.toLowerCase().includes(filterQuality))
    .sort((a, b) => {
      switch (sortBy) {
        case 'score': return (b.score || 0) - (a.score || 0);
        case 'size': return b.size - a.size;
        case 'seeders': return b.seeders - a.seeders;
        case 'date': return new Date(b.publishDate).getTime() - new Date(a.publishDate).getTime();
        default: return 0;
      }
    });

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0">
        {/* Background overlay */}
        <div 
          className="fixed inset-0 bg-black bg-opacity-50 transition-opacity"
          onClick={onClose}
        />

        {/* Modal panel */}
        <div className="inline-block align-bottom bg-white dark:bg-secondary-800 rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-5xl sm:w-full">
          {/* Header */}
          <div className="px-6 py-4 border-b border-secondary-200 dark:border-secondary-700">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-xl font-bold text-secondary-900 dark:text-white">
                  Search Releases
                </h2>
                <p className="text-sm text-secondary-600 dark:text-secondary-400 mt-1">
                  Searching for: {movieTitle}
                </p>
              </div>
              <button
                onClick={onClose}
                className="p-2 rounded-full hover:bg-secondary-100 dark:hover:bg-secondary-700"
              >
                <XMarkIcon className="h-6 w-6" />
              </button>
            </div>
          </div>

          {/* Search Controls */}
          <div className="px-6 py-4 bg-secondary-50 dark:bg-secondary-900 border-b border-secondary-200 dark:border-secondary-700">
            <div className="space-y-4">
              {/* Indexer Selection */}
              <div>
                <label className="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-2">
                  Indexers
                </label>
                <div className="flex flex-wrap gap-2">
                  {['HDBits', 'Rarbg', 'Nyaa', '1337x', 'YTS'].map(indexer => (
                    <button
                      key={indexer}
                      onClick={() => {
                        const newSet = new Set(selectedIndexers);
                        const key = indexer.toLowerCase();
                        if (newSet.has(key)) {
                          newSet.delete(key);
                        } else {
                          newSet.add(key);
                        }
                        setSelectedIndexers(newSet);
                      }}
                      className={`px-3 py-1 rounded-full text-sm font-medium transition-colors ${
                        selectedIndexers.has(indexer.toLowerCase())
                          ? 'bg-primary-600 text-white'
                          : 'bg-secondary-200 text-secondary-700 dark:bg-secondary-700 dark:text-secondary-300 hover:bg-secondary-300 dark:hover:bg-secondary-600'
                      }`}
                    >
                      <ServerStackIcon className="h-4 w-4 inline mr-1" />
                      {indexer}
                    </button>
                  ))}
                </div>
              </div>

              {/* Filters and Sort */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div>
                  <label className="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">
                    Quality Filter
                  </label>
                  <select
                    value={filterQuality}
                    onChange={(e) => setFilterQuality(e.target.value)}
                    className="form-input"
                  >
                    <option value="all">All Qualities</option>
                    <option value="2160p">2160p (4K)</option>
                    <option value="1080p">1080p</option>
                    <option value="720p">720p</option>
                    <option value="bluray">BluRay</option>
                    <option value="web">WEB</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-1">
                    Sort By
                  </label>
                  <select
                    value={sortBy}
                    onChange={(e) => setSortBy(e.target.value as any)}
                    className="form-input"
                  >
                    <option value="score">Best Match</option>
                    <option value="seeders">Most Seeders</option>
                    <option value="size">Largest Size</option>
                    <option value="date">Newest</option>
                  </select>
                </div>
                <div className="flex items-end">
                  <LoadingButton
                    onClick={handleSearch}
                    loading={isSearching}
                    loadingText="Searching..."
                    variant="primary"
                    className="w-full justify-center"
                  >
                    <MagnifyingGlassIcon className="h-5 w-5 mr-2" />
                    Search Indexers
                  </LoadingButton>
                </div>
              </div>
            </div>
          </div>

          {/* Results */}
          <div className="px-6 py-4 max-h-96 overflow-y-auto">
            {isSearching ? (
              <div className="text-center py-8">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
                <p className="text-secondary-600 dark:text-secondary-400 mt-4">
                  Searching indexers...
                </p>
              </div>
            ) : filteredResults.length === 0 ? (
              <div className="text-center py-8">
                <DocumentMagnifyingGlassIcon className="h-16 w-16 text-secondary-400 mx-auto mb-4" />
                <p className="text-secondary-600 dark:text-secondary-400">
                  {searchResults.length === 0 
                    ? 'No results yet. Click "Search Indexers" to find releases.'
                    : 'No releases match your filters.'}
                </p>
              </div>
            ) : (
              <div className="space-y-3">
                {filteredResults.map((release) => (
                  <div key={release.id} className="card-interactive p-4">
                    <div className="flex items-start justify-between">
                      <div className="flex-1 min-w-0">
                        {/* Title and Quality */}
                        <div className="flex items-start space-x-3">
                          <FilmIcon className="h-5 w-5 text-secondary-400 mt-0.5" />
                          <div className="flex-1">
                            <h4 className="text-sm font-medium text-secondary-900 dark:text-white break-all">
                              {release.title}
                            </h4>
                            
                            {/* Metadata Row */}
                            <div className="flex flex-wrap items-center gap-3 mt-2 text-xs text-secondary-600 dark:text-secondary-400">
                              <span className={`px-2 py-1 rounded-full font-medium ${getQualityColor(release.quality)}`}>
                                {release.quality}
                              </span>
                              <span className="flex items-center">
                                <ServerStackIcon className="h-3 w-3 mr-1" />
                                {release.indexer}
                              </span>
                              <span>{formatSize(release.size)}</span>
                              <span className="flex items-center">
                                <ArrowDownTrayIcon className="h-3 w-3 mr-1 text-success-600" />
                                {release.seeders}
                              </span>
                              <span className="flex items-center">
                                <ClockIcon className="h-3 w-3 mr-1" />
                                {formatDate(release.publishDate)}
                              </span>
                              {release.sceneGroup && (
                                <span className="flex items-center">
                                  <UserGroupIcon className="h-3 w-3 mr-1" />
                                  {release.sceneGroup}
                                </span>
                              )}
                              {release.languages && (
                                <span className="flex items-center">
                                  <TagIcon className="h-3 w-3 mr-1" />
                                  {release.languages.join(', ')}
                                </span>
                              )}
                            </div>

                            {/* Score Indicator */}
                            {release.score && (
                              <div className="mt-2">
                                <div className="flex items-center space-x-2">
                                  <span className="text-xs text-secondary-500">Match Score:</span>
                                  <div className="flex-1 max-w-xs bg-secondary-200 dark:bg-secondary-700 rounded-full h-2">
                                    <div 
                                      className={`h-2 rounded-full ${
                                        release.score >= 90 ? 'bg-success-500' :
                                        release.score >= 70 ? 'bg-warning-500' : 'bg-error-500'
                                      }`}
                                      style={{ width: `${release.score}%` }}
                                    />
                                  </div>
                                  <span className={`text-xs font-medium ${getScoreColor(release.score)}`}>
                                    {release.score}%
                                  </span>
                                </div>
                              </div>
                            )}
                          </div>
                        </div>
                      </div>

                      {/* Download Button */}
                      <div className="ml-4">
                        <LoadingButton
                          onClick={() => handleDownload(release)}
                          loading={downloadingReleases.has(release.id)}
                          loadingText="Starting..."
                          variant="primary"
                          size="sm"
                        >
                          <ArrowDownTrayIcon className="h-4 w-4 mr-1" />
                          Download
                        </LoadingButton>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="px-6 py-4 bg-secondary-50 dark:bg-secondary-900 flex justify-between items-center">
            <p className="text-sm text-secondary-600 dark:text-secondary-400">
              {filteredResults.length} release{filteredResults.length !== 1 ? 's' : ''} found
            </p>
            <button
              onClick={onClose}
              className="btn-ghost"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};