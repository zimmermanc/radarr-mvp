import React, { useState, useEffect } from 'react';
import {
  ArrowDownTrayIcon,
  PauseIcon,
  PlayIcon,
  TrashIcon,
  ArrowPathIcon,
  ChevronUpIcon,
  ChevronDownIcon,
  ExclamationTriangleIcon,
  CheckCircleIcon,
  ClockIcon,
  ServerStackIcon,
  DocumentArrowDownIcon
} from '@heroicons/react/24/outline';
import { usePageTitle } from '../contexts/UIContext';
import { useToast } from '../components/ui/Toast';
import { LoadingButton } from '../components/ui/Loading';
import { useQueueUpdates, useDownloadUpdates } from '../contexts/WebSocketContext';

interface QueueItem {
  id: string;
  movieId: number;
  movieTitle: string;
  quality: string;
  protocol: 'torrent' | 'usenet';
  indexer: string;
  downloadClient: string;
  status: 'queued' | 'downloading' | 'paused' | 'completed' | 'failed' | 'importing';
  size: number;
  sizeLeft: number;
  timeleft?: string;
  estimatedCompletionTime?: string;
  downloadedSize: number;
  progress: number;
  downloadRate?: number;
  uploadRate?: number;
  seeders?: number;
  leechers?: number;
  eta?: string;
  errorMessage?: string;
  trackedDownloadStatus?: string;
  trackedDownloadState?: string;
  statusMessages?: string[];
  outputPath?: string;
  downloadId?: string;
  added: string;
}

export const Queue: React.FC = () => {
  usePageTitle('Download Queue');
  
  const [queueItems, setQueueItems] = useState<QueueItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [selectedItems, setSelectedItems] = useState<Set<string>>(new Set());
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [sortBy, setSortBy] = useState<'progress' | 'eta' | 'added'>('added');
  
  const { success, error: toastError } = useToast();

  // Mock queue data
  const mockQueueItems: QueueItem[] = [
    {
      id: '1',
      movieId: 1,
      movieTitle: 'Inception',
      quality: '1080p BluRay',
      protocol: 'torrent',
      indexer: 'HDBits',
      downloadClient: 'qBittorrent',
      status: 'downloading',
      size: 10737418240, // 10GB
      sizeLeft: 3221225472, // 3GB
      downloadedSize: 7516192768, // 7GB
      progress: 70,
      downloadRate: 5242880, // 5MB/s
      uploadRate: 1048576, // 1MB/s
      seeders: 45,
      leechers: 2,
      eta: '10 min',
      added: new Date(Date.now() - 1800000).toISOString()
    },
    {
      id: '2',
      movieId: 2,
      movieTitle: 'The Dark Knight',
      quality: '2160p WEB-DL',
      protocol: 'torrent',
      indexer: 'HDBits',
      downloadClient: 'qBittorrent',
      status: 'downloading',
      size: 21474836480, // 20GB
      sizeLeft: 10737418240, // 10GB
      downloadedSize: 10737418240, // 10GB
      progress: 50,
      downloadRate: 10485760, // 10MB/s
      uploadRate: 2097152, // 2MB/s
      seeders: 32,
      leechers: 5,
      eta: '17 min',
      added: new Date(Date.now() - 3600000).toISOString()
    },
    {
      id: '3',
      movieId: 3,
      movieTitle: 'Interstellar',
      quality: '1080p BluRay',
      protocol: 'torrent',
      indexer: 'Rarbg',
      downloadClient: 'qBittorrent',
      status: 'paused',
      size: 15032385536, // 14GB
      sizeLeft: 7516192768, // 7GB
      downloadedSize: 7516192768, // 7GB
      progress: 50,
      downloadRate: 0,
      uploadRate: 0,
      seeders: 20,
      leechers: 8,
      added: new Date(Date.now() - 7200000).toISOString()
    },
    {
      id: '4',
      movieId: 4,
      movieTitle: 'Dune',
      quality: '1080p WEB-DL',
      protocol: 'torrent',
      indexer: 'YTS',
      downloadClient: 'qBittorrent',
      status: 'completed',
      size: 2147483648, // 2GB
      sizeLeft: 0,
      downloadedSize: 2147483648,
      progress: 100,
      downloadRate: 0,
      uploadRate: 524288, // 512KB/s
      seeders: 120,
      leechers: 15,
      added: new Date(Date.now() - 10800000).toISOString()
    },
    {
      id: '5',
      movieId: 5,
      movieTitle: 'The Matrix',
      quality: '720p BluRay',
      protocol: 'torrent',
      indexer: 'HDBits',
      downloadClient: 'qBittorrent',
      status: 'failed',
      size: 4294967296, // 4GB
      sizeLeft: 3221225472, // 3GB
      downloadedSize: 1073741824, // 1GB
      progress: 25,
      downloadRate: 0,
      uploadRate: 0,
      errorMessage: 'No seeders available',
      added: new Date(Date.now() - 14400000).toISOString()
    }
  ];

  // Handle WebSocket updates
  useQueueUpdates((update) => {
    console.log('Queue update received:', update);
    loadQueue(true);
  });

  useDownloadUpdates((update) => {
    console.log('Download update received:', update);
    // Update specific item in queue based on update type
    if (update.type === 'progress') {
      setQueueItems(prev => prev.map(item => 
        item.id === update.itemId 
          ? { ...item, progress: update.progress, downloadRate: update.speed }
          : item
      ));
    } else {
      loadQueue(true);
    }
  });

  useEffect(() => {
    loadQueue();
    // Set up auto-refresh every 5 seconds as fallback
    const interval = setInterval(() => {
      loadQueue(true);
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  const loadQueue = async (isRefresh = false) => {
    if (!isRefresh) setLoading(true);
    else setRefreshing(true);
    
    try {
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 500));
      
      // TODO: Replace with actual API call
      // const response = await radarrApi.getQueue();
      
      setQueueItems(mockQueueItems);
    } catch (err) {
      toastError('Error', 'Failed to load download queue');
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  const handlePause = async (item: QueueItem) => {
    try {
      // TODO: Implement pause API call
      success('Download Paused', `Paused: ${item.movieTitle}`);
      loadQueue();
    } catch (err) {
      toastError('Error', 'Failed to pause download');
    }
  };

  const handleResume = async (item: QueueItem) => {
    try {
      // TODO: Implement resume API call
      success('Download Resumed', `Resumed: ${item.movieTitle}`);
      loadQueue();
    } catch (err) {
      toastError('Error', 'Failed to resume download');
    }
  };

  const handleRemove = async (_itemId: string) => {
    try {
      // TODO: Implement remove API call
      success('Download Removed', 'Item removed from queue');
      loadQueue();
    } catch (err) {
      toastError('Error', 'Failed to remove download');
    }
  };

  const handleBulkAction = async (action: 'pause' | 'resume' | 'remove') => {
    try {
      // TODO: Implement bulk action API calls
      const count = selectedItems.size;
      success(`Bulk ${action}`, `${count} items ${action}d`);
      setSelectedItems(new Set());
      loadQueue();
    } catch (err) {
      toastError('Error', `Failed to ${action} selected items`);
    }
  };

  const handlePriorityChange = async (_itemId: string, direction: 'up' | 'down') => {
    try {
      // TODO: Implement priority API call
      success('Priority Changed', `Moved item ${direction}`);
      loadQueue();
    } catch (err) {
      toastError('Error', 'Failed to change priority');
    }
  };

  const formatSize = (bytes: number): string => {
    const gb = bytes / (1024 * 1024 * 1024);
    if (gb >= 1) return `${gb.toFixed(2)} GB`;
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(0)} MB`;
  };

  const formatSpeed = (bytesPerSecond: number): string => {
    if (!bytesPerSecond) return '0 KB/s';
    const mbps = bytesPerSecond / (1024 * 1024);
    if (mbps >= 1) return `${mbps.toFixed(1)} MB/s`;
    const kbps = bytesPerSecond / 1024;
    return `${kbps.toFixed(0)} KB/s`;
  };

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    
    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    return date.toLocaleDateString();
  };

  const getStatusIcon = (status: QueueItem['status']) => {
    switch (status) {
      case 'downloading':
        return <ArrowDownTrayIcon className="h-5 w-5 text-primary-600 animate-bounce" />;
      case 'paused':
        return <PauseIcon className="h-5 w-5 text-warning-600" />;
      case 'completed':
        return <CheckCircleIcon className="h-5 w-5 text-success-600" />;
      case 'failed':
        return <ExclamationTriangleIcon className="h-5 w-5 text-error-600" />;
      case 'importing':
        return <DocumentArrowDownIcon className="h-5 w-5 text-info-600 animate-pulse" />;
      default:
        return <ClockIcon className="h-5 w-5 text-secondary-400" />;
    }
  };

  const getStatusColor = (status: QueueItem['status']) => {
    switch (status) {
      case 'downloading': return 'text-primary-600 bg-primary-100 dark:bg-primary-900/30';
      case 'paused': return 'text-warning-600 bg-warning-100 dark:bg-warning-900/30';
      case 'completed': return 'text-success-600 bg-success-100 dark:bg-success-900/30';
      case 'failed': return 'text-error-600 bg-error-100 dark:bg-error-900/30';
      case 'importing': return 'text-info-600 bg-info-100 dark:bg-info-900/30';
      default: return 'text-secondary-600 bg-secondary-100 dark:bg-secondary-700';
    }
  };

  const filteredItems = queueItems
    .filter(item => filterStatus === 'all' || item.status === filterStatus)
    .sort((a, b) => {
      switch (sortBy) {
        case 'progress': return b.progress - a.progress;
        case 'eta': 
          const aEta = parseInt(a.eta || '999');
          const bEta = parseInt(b.eta || '999');
          return aEta - bEta;
        case 'added': 
          return new Date(b.added).getTime() - new Date(a.added).getTime();
        default: return 0;
      }
    });

  if (loading) {
    return (
      <div className="p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-secondary-200 dark:bg-secondary-700 rounded w-1/4"></div>
          <div className="h-32 bg-secondary-200 dark:bg-secondary-700 rounded"></div>
          <div className="h-32 bg-secondary-200 dark:bg-secondary-700 rounded"></div>
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
            Download Queue
          </h1>
          <p className="text-secondary-600 dark:text-secondary-400">
            {queueItems.length} items in queue
            {selectedItems.size > 0 && ` • ${selectedItems.size} selected`}
          </p>
        </div>
        <LoadingButton
          onClick={() => loadQueue(true)}
          loading={refreshing}
          variant="secondary"
        >
          <ArrowPathIcon className={`h-5 w-5 mr-2 ${refreshing ? 'animate-spin' : ''}`} />
          Refresh
        </LoadingButton>
      </div>

      {/* Bulk Actions */}
      {selectedItems.size > 0 && (
        <div className="card p-4 bg-primary-50 dark:bg-primary-900/20 border-primary-200 dark:border-primary-800">
          <div className="flex items-center justify-between">
            <span className="text-sm text-primary-700 dark:text-primary-300">
              {selectedItems.size} items selected
            </span>
            <div className="flex items-center space-x-2">
              <button
                onClick={() => handleBulkAction('pause')}
                className="btn-secondary text-sm"
              >
                <PauseIcon className="h-4 w-4 mr-1" />
                Pause All
              </button>
              <button
                onClick={() => handleBulkAction('resume')}
                className="btn-primary text-sm"
              >
                <PlayIcon className="h-4 w-4 mr-1" />
                Resume All
              </button>
              <button
                onClick={() => handleBulkAction('remove')}
                className="btn-danger text-sm"
              >
                <TrashIcon className="h-4 w-4 mr-1" />
                Remove All
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Filters */}
      <div className="card p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <div>
              <label className="text-sm text-secondary-600 dark:text-secondary-400 mr-2">
                Status:
              </label>
              <select
                value={filterStatus}
                onChange={(e) => setFilterStatus(e.target.value)}
                className="form-input py-1"
              >
                <option value="all">All</option>
                <option value="downloading">Downloading</option>
                <option value="paused">Paused</option>
                <option value="completed">Completed</option>
                <option value="failed">Failed</option>
                <option value="importing">Importing</option>
              </select>
            </div>
            <div>
              <label className="text-sm text-secondary-600 dark:text-secondary-400 mr-2">
                Sort:
              </label>
              <select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as any)}
                className="form-input py-1"
              >
                <option value="added">Date Added</option>
                <option value="progress">Progress</option>
                <option value="eta">ETA</option>
              </select>
            </div>
          </div>
          <div className="text-sm text-secondary-600 dark:text-secondary-400">
            {filteredItems.length} items
          </div>
        </div>
      </div>

      {/* Queue Items */}
      {filteredItems.length === 0 ? (
        <div className="card p-8 text-center">
          <ArrowDownTrayIcon className="h-16 w-16 text-secondary-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-secondary-900 dark:text-white mb-2">
            No downloads in queue
          </h3>
          <p className="text-secondary-600 dark:text-secondary-400">
            Search for movies to start downloading
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {filteredItems.map((item, index) => (
            <div key={item.id} className="card-interactive p-4">
              <div className="flex items-start space-x-4">
                {/* Selection checkbox */}
                <div className="flex-shrink-0 pt-1">
                  <input
                    type="checkbox"
                    checked={selectedItems.has(item.id)}
                    onChange={() => {
                      const newSet = new Set(selectedItems);
                      if (newSet.has(item.id)) {
                        newSet.delete(item.id);
                      } else {
                        newSet.add(item.id);
                      }
                      setSelectedItems(newSet);
                    }}
                    className="h-4 w-4 text-primary-600 rounded border-secondary-300 focus:ring-primary-500"
                  />
                </div>

                {/* Status Icon */}
                <div className="flex-shrink-0 pt-1">
                  {getStatusIcon(item.status)}
                </div>

                {/* Content */}
                <div className="flex-1 min-w-0">
                  {/* Title and Info */}
                  <div className="flex items-start justify-between">
                    <div>
                      <h3 className="text-base font-semibold text-secondary-900 dark:text-white">
                        {item.movieTitle}
                      </h3>
                      <div className="flex flex-wrap items-center gap-3 mt-1 text-sm text-secondary-600 dark:text-secondary-400">
                        <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${getStatusColor(item.status)}`}>
                          {item.status}
                        </span>
                        <span>{item.quality}</span>
                        <span className="flex items-center">
                          <ServerStackIcon className="h-4 w-4 mr-1" />
                          {item.indexer}
                        </span>
                        <span>{formatSize(item.size)}</span>
                        <span>Added {formatDate(item.added)}</span>
                      </div>
                    </div>

                    {/* Priority Controls */}
                    <div className="flex items-center space-x-1 ml-4">
                      <button
                        onClick={() => handlePriorityChange(item.id, 'up')}
                        className="p-1 hover:bg-secondary-100 dark:hover:bg-secondary-700 rounded"
                        disabled={index === 0}
                      >
                        <ChevronUpIcon className="h-4 w-4" />
                      </button>
                      <button
                        onClick={() => handlePriorityChange(item.id, 'down')}
                        className="p-1 hover:bg-secondary-100 dark:hover:bg-secondary-700 rounded"
                        disabled={index === filteredItems.length - 1}
                      >
                        <ChevronDownIcon className="h-4 w-4" />
                      </button>
                    </div>
                  </div>

                  {/* Progress Bar */}
                  {(item.status === 'downloading' || item.status === 'paused') && (
                    <div className="mt-3">
                      <div className="flex items-center justify-between text-sm mb-1">
                        <span className="text-secondary-600 dark:text-secondary-400">
                          {formatSize(item.downloadedSize)} / {formatSize(item.size)}
                        </span>
                        <span className="font-medium text-secondary-900 dark:text-white">
                          {item.progress}%
                        </span>
                      </div>
                      <div className="w-full bg-secondary-200 dark:bg-secondary-700 rounded-full h-2">
                        <div 
                          className={`h-2 rounded-full transition-all ${
                            item.status === 'paused' ? 'bg-warning-500' : 'bg-primary-600'
                          }`}
                          style={{ width: `${item.progress}%` }}
                        />
                      </div>
                    </div>
                  )}

                  {/* Download Stats */}
                  {item.status === 'downloading' && (
                    <div className="flex items-center space-x-4 mt-2 text-sm text-secondary-600 dark:text-secondary-400">
                      <span>↓ {formatSpeed(item.downloadRate || 0)}</span>
                      <span>↑ {formatSpeed(item.uploadRate || 0)}</span>
                      {item.seeders !== undefined && (
                        <span>Seeds: {item.seeders}</span>
                      )}
                      {item.leechers !== undefined && (
                        <span>Peers: {item.leechers}</span>
                      )}
                      {item.eta && (
                        <span className="text-primary-600 font-medium">
                          ETA: {item.eta}
                        </span>
                      )}
                    </div>
                  )}

                  {/* Error Message */}
                  {item.errorMessage && (
                    <div className="mt-2 p-2 bg-error-50 dark:bg-error-900/20 rounded text-sm text-error-700 dark:text-error-300">
                      {item.errorMessage}
                    </div>
                  )}
                </div>

                {/* Actions */}
                <div className="flex-shrink-0 flex items-center space-x-2">
                  {item.status === 'downloading' && (
                    <button
                      onClick={() => handlePause(item)}
                      className="btn-ghost p-2"
                      title="Pause"
                    >
                      <PauseIcon className="h-5 w-5" />
                    </button>
                  )}
                  {item.status === 'paused' && (
                    <button
                      onClick={() => handleResume(item)}
                      className="btn-ghost p-2"
                      title="Resume"
                    >
                      <PlayIcon className="h-5 w-5" />
                    </button>
                  )}
                  <button
                    onClick={() => handleRemove(item.id)}
                    className="btn-ghost p-2 text-error-600 hover:bg-error-50 dark:hover:bg-error-900/20"
                    title="Remove"
                  >
                    <TrashIcon className="h-5 w-5" />
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};