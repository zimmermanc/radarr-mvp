import React, { useState, useEffect } from 'react';
import { 
  CogIcon,
  ServerIcon,
  KeyIcon,
  CheckCircleIcon,
  ExclamationTriangleIcon,
  ArrowPathIcon,
  ServerStackIcon,
  ArrowDownTrayIcon,
  FolderIcon,
  BellIcon
} from '@heroicons/react/24/outline';
import { radarrApi } from '../lib/api';
import { usePageTitle } from '../contexts/UIContext';
// import { useToast } from '../components/ui/Toast'; // Currently unused

export const Settings: React.FC = () => {
  usePageTitle('Settings');

  const [apiUrl, setApiUrl] = useState(import.meta.env.VITE_API_BASE_URL || 'http://localhost:7878');
  const [apiKey, setApiKey] = useState(import.meta.env.VITE_API_KEY || 'mysecurekey123');
  const [testing, setTesting] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<'unknown' | 'success' | 'error'>('unknown');
  const [statusMessage, setStatusMessage] = useState('');
  const [saved, setSaved] = useState(false);
  const [activeTab, setActiveTab] = useState<'general' | 'indexers' | 'download' | 'media' | 'notifications'>('general');
  
  // Indexer settings
  const [hdbitsEnabled, setHdbitsEnabled] = useState(true);
  const [hdbitsUsername, setHdbitsUsername] = useState('');
  const [hdbitsPassword, setHdbitsPassword] = useState('');
  
  // Download client settings
  const [qbitEnabled, setQbitEnabled] = useState(true);
  const [qbitUrl, setQbitUrl] = useState('http://localhost:8080');
  const [qbitUsername, setQbitUsername] = useState('admin');
  const [qbitPassword, setQbitPassword] = useState('admin');
  
  // Media settings
  const [mediaRoot, setMediaRoot] = useState('/media/movies');
  const [fileNaming, setFileNaming] = useState('{Movie Title} ({Year}) - {Quality}');
  const [replaceSpaces, setReplaceSpaces] = useState(false);
  const [createFolders, setCreateFolders] = useState(true);

  // const { success } = useToast(); // Currently unused

  useEffect(() => {
    testConnection();
  }, []);

  const testConnection = async () => {
    setTesting(true);
    setStatusMessage('');
    
    try {
      // Update the API client with current settings
      radarrApi.updateConfig({
        baseUrl: apiUrl,
        apiKey: apiKey,
      });

      const isConnected = await radarrApi.testConnection();
      
      if (isConnected) {
        setConnectionStatus('success');
        setStatusMessage('Successfully connected to Radarr API');
      } else {
        setConnectionStatus('error');
        setStatusMessage('Failed to connect to Radarr API');
      }
    } catch (error) {
      setConnectionStatus('error');
      setStatusMessage(error instanceof Error ? error.message : 'Connection test failed');
    } finally {
      setTesting(false);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    
    // Update the API client configuration
    radarrApi.updateConfig({
      baseUrl: apiUrl,
      apiKey: apiKey,
    });

    // Test the new connection
    await testConnection();
    
    // Show saved confirmation
    setSaved(true);
    setTimeout(() => setSaved(false), 3000);
  };

  const handleReset = () => {
    setApiUrl(import.meta.env.VITE_API_BASE_URL || 'http://localhost:7878');
    setApiKey(import.meta.env.VITE_API_KEY || 'mysecurekey123');
    setConnectionStatus('unknown');
    setStatusMessage('');
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-secondary-900 dark:text-white">
          Settings
        </h1>
        <p className="text-secondary-600 dark:text-secondary-400">
          Configure your Radarr API connection and application preferences
        </p>
      </div>


      {/* Tab Navigation */}
      <div className="card p-1">
        <div className="flex space-x-1">
          {[
            { id: 'general', label: 'General', icon: CogIcon },
            { id: 'indexers', label: 'Indexers', icon: ServerStackIcon },
            { id: 'download', label: 'Download Clients', icon: ArrowDownTrayIcon },
            { id: 'media', label: 'Media Management', icon: FolderIcon },
            { id: 'notifications', label: 'Notifications', icon: BellIcon }
          ].map((tab) => {
            const Icon = tab.icon;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as any)}
                className={`flex items-center px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                  activeTab === tab.id
                    ? 'bg-primary-600 text-white'
                    : 'text-secondary-600 dark:text-secondary-400 hover:bg-secondary-100 dark:hover:bg-secondary-700'
                }`}
              >
                <Icon className="h-4 w-4 mr-2" />
                {tab.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* General Settings (existing API config) */}
      {activeTab === 'general' && (
        <>
          {/* Existing API Configuration card */}
          <div className="card p-6">
            <div className="flex items-center mb-4">
              <ServerIcon className="h-6 w-6 text-primary-600 mr-3" />
              <h2 className="text-lg font-semibold text-secondary-900 dark:text-white">
                API Configuration
              </h2>
            </div>

            <form onSubmit={handleSave} className="space-y-4">
              <div>
                <label className="form-label">
                  API Base URL
                </label>
                <input
                  type="url"
                  value={apiUrl}
                  onChange={(e) => setApiUrl(e.target.value)}
                  placeholder="http://localhost:7878"
                  className="form-input"
                  required
                />
                <p className="text-xs text-secondary-500 dark:text-secondary-400 mt-1">
                  The base URL where your Radarr instance is running
                </p>
              </div>

              <div>
                <label className="form-label">
                  API Key
                </label>
                <div className="relative">
                  <KeyIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 h-5 w-5 text-secondary-400" />
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder="Your Radarr API key"
                    className="form-input pl-10"
                    required
                  />
                </div>
                <p className="text-xs text-secondary-500 dark:text-secondary-400 mt-1">
                  Found in Radarr Settings → General → Security → API Key
                </p>
              </div>

              {/* Connection Status */}
              <div className="bg-secondary-50 dark:bg-secondary-800 rounded-lg p-4">
                <div className="flex items-center justify-between mb-3">
                  <span className="text-sm font-medium text-secondary-700 dark:text-secondary-300">
                    Connection Status
                  </span>
                  <button
                    type="button"
                    onClick={testConnection}
                    disabled={testing}
                    className="btn-ghost text-sm"
                  >
                    {testing ? (
                      <ArrowPathIcon className="h-4 w-4 animate-spin" />
                    ) : (
                      <>
                        <ArrowPathIcon className="h-4 w-4 mr-1" />
                        Test
                      </>
                    )}
                  </button>
                </div>
                
                {connectionStatus !== 'unknown' && (
                  <div className={`flex items-center ${
                    connectionStatus === 'success' 
                      ? 'text-success-600' 
                      : 'text-error-600'
                  }`}>
                    {connectionStatus === 'success' ? (
                      <CheckCircleIcon className="h-5 w-5 mr-2" />
                    ) : (
                      <ExclamationTriangleIcon className="h-5 w-5 mr-2" />
                    )}
                    <span className="text-sm">{statusMessage}</span>
                  </div>
                )}
                
                {testing && (
                  <div className="flex items-center text-secondary-600 dark:text-secondary-400">
                    <div className="animate-spin h-4 w-4 border-2 border-current border-t-transparent rounded-full mr-2"></div>
                    <span className="text-sm">Testing connection...</span>
                  </div>
                )}
              </div>

              {/* Actions */}
              <div className="flex items-center justify-between pt-4 border-t border-secondary-200 dark:border-secondary-600">
                <button
                  type="button"
                  onClick={handleReset}
                  className="btn-secondary"
                >
                  Reset to Defaults
                </button>
                
                <div className="flex items-center space-x-3">
                  {saved && (
                    <div className="flex items-center text-success-600 text-sm">
                      <CheckCircleIcon className="h-4 w-4 mr-1" />
                      Settings saved
                    </div>
                  )}
                  <button
                    type="submit"
                    className="btn-primary"
                  >
                    Save Settings
                  </button>
                </div>
              </div>
            </form>
          </div>
        </>
      )}

      {/* Indexer Settings */}
      {activeTab === 'indexers' && (
        <div className="card p-6">
          <div className="flex items-center mb-4">
            <ServerStackIcon className="h-6 w-6 text-primary-600 mr-3" />
            <h2 className="text-lg font-semibold text-secondary-900 dark:text-white">
              Indexer Configuration
            </h2>
          </div>

          <div className="space-y-6">
            {/* HDBits Configuration */}
            <div className="border border-secondary-200 dark:border-secondary-600 rounded-lg p-4">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-medium text-secondary-900 dark:text-white">HDBits</h3>
                <label className="flex items-center">
                  <input
                    type="checkbox"
                    checked={hdbitsEnabled}
                    onChange={(e) => setHdbitsEnabled(e.target.checked)}
                    className="form-checkbox h-4 w-4 text-primary-600"
                  />
                  <span className="ml-2 text-sm">Enabled</span>
                </label>
              </div>
              
              {hdbitsEnabled && (
                <div className="space-y-3">
                  <div>
                    <label className="form-label text-sm">Username</label>
                    <input
                      type="text"
                      value={hdbitsUsername}
                      onChange={(e) => setHdbitsUsername(e.target.value)}
                      placeholder="HDBits username"
                      className="form-input"
                    />
                  </div>
                  <div>
                    <label className="form-label text-sm">Password</label>
                    <input
                      type="password"
                      value={hdbitsPassword}
                      onChange={(e) => setHdbitsPassword(e.target.value)}
                      placeholder="HDBits password"
                      className="form-input"
                    />
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-secondary-600 dark:text-secondary-400">
                      Rate Limit: 2 seconds between requests
                    </span>
                    <button className="btn-ghost text-sm">Test Connection</button>
                  </div>
                </div>
              )}
            </div>

            {/* Add more indexers button */}
            <button className="btn-secondary w-full">
              <ServerStackIcon className="h-5 w-5 mr-2" />
              Add Indexer
            </button>
          </div>
        </div>
      )}

      {/* Download Client Settings */}
      {activeTab === 'download' && (
        <div className="card p-6">
          <div className="flex items-center mb-4">
            <ArrowDownTrayIcon className="h-6 w-6 text-primary-600 mr-3" />
            <h2 className="text-lg font-semibold text-secondary-900 dark:text-white">
              Download Client Configuration
            </h2>
          </div>

          <div className="space-y-6">
            {/* qBittorrent Configuration */}
            <div className="border border-secondary-200 dark:border-secondary-600 rounded-lg p-4">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-medium text-secondary-900 dark:text-white">qBittorrent</h3>
                <label className="flex items-center">
                  <input
                    type="checkbox"
                    checked={qbitEnabled}
                    onChange={(e) => setQbitEnabled(e.target.checked)}
                    className="form-checkbox h-4 w-4 text-primary-600"
                  />
                  <span className="ml-2 text-sm">Enabled</span>
                </label>
              </div>
              
              {qbitEnabled && (
                <div className="space-y-3">
                  <div>
                    <label className="form-label text-sm">URL</label>
                    <input
                      type="url"
                      value={qbitUrl}
                      onChange={(e) => setQbitUrl(e.target.value)}
                      placeholder="http://localhost:8080"
                      className="form-input"
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="form-label text-sm">Username</label>
                      <input
                        type="text"
                        value={qbitUsername}
                        onChange={(e) => setQbitUsername(e.target.value)}
                        placeholder="admin"
                        className="form-input"
                      />
                    </div>
                    <div>
                      <label className="form-label text-sm">Password</label>
                      <input
                        type="password"
                        value={qbitPassword}
                        onChange={(e) => setQbitPassword(e.target.value)}
                        placeholder="Password"
                        className="form-input"
                      />
                    </div>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-secondary-600 dark:text-secondary-400">
                      Category: radarr
                    </span>
                    <button className="btn-ghost text-sm">Test Connection</button>
                  </div>
                </div>
              )}
            </div>

            {/* Add more download clients button */}
            <button className="btn-secondary w-full">
              <ArrowDownTrayIcon className="h-5 w-5 mr-2" />
              Add Download Client
            </button>
          </div>
        </div>
      )}

      {/* Media Management Settings */}
      {activeTab === 'media' && (
        <div className="card p-6">
          <div className="flex items-center mb-4">
            <FolderIcon className="h-6 w-6 text-primary-600 mr-3" />
            <h2 className="text-lg font-semibold text-secondary-900 dark:text-white">
              Media Management
            </h2>
          </div>

          <div className="space-y-4">
            <div>
              <label className="form-label">Root Folder</label>
              <input
                type="text"
                value={mediaRoot}
                onChange={(e) => setMediaRoot(e.target.value)}
                placeholder="/media/movies"
                className="form-input"
              />
              <p className="text-xs text-secondary-500 dark:text-secondary-400 mt-1">
                The folder where your movie library is stored
              </p>
            </div>

            <div>
              <label className="form-label">File Naming Template</label>
              <input
                type="text"
                value={fileNaming}
                onChange={(e) => setFileNaming(e.target.value)}
                className="form-input font-mono text-sm"
              />
              <p className="text-xs text-secondary-500 dark:text-secondary-400 mt-1">
                Available tokens: {'{Movie Title}'}, {'{Year}'}, {'{Quality}'}, {'{Resolution}'}
              </p>
            </div>

            <div className="space-y-3">
              <label className="flex items-center">
                <input
                  type="checkbox"
                  checked={replaceSpaces}
                  onChange={(e) => setReplaceSpaces(e.target.checked)}
                  className="form-checkbox h-4 w-4 text-primary-600"
                />
                <span className="ml-2 text-sm">Replace spaces with underscores</span>
              </label>
              
              <label className="flex items-center">
                <input
                  type="checkbox"
                  checked={createFolders}
                  onChange={(e) => setCreateFolders(e.target.checked)}
                  className="form-checkbox h-4 w-4 text-primary-600"
                />
                <span className="ml-2 text-sm">Create movie folders</span>
              </label>
            </div>

            <div className="pt-4 border-t border-secondary-200 dark:border-secondary-600">
              <h4 className="text-sm font-medium text-secondary-900 dark:text-white mb-2">
                File Management
              </h4>
              <div className="space-y-2 text-sm">
                <label className="flex items-center">
                  <input type="checkbox" className="form-checkbox h-4 w-4 text-primary-600" defaultChecked />
                  <span className="ml-2">Rename movies</span>
                </label>
                <label className="flex items-center">
                  <input type="checkbox" className="form-checkbox h-4 w-4 text-primary-600" defaultChecked />
                  <span className="ml-2">Use hardlinks instead of copy</span>
                </label>
                <label className="flex items-center">
                  <input type="checkbox" className="form-checkbox h-4 w-4 text-primary-600" />
                  <span className="ml-2">Delete empty folders</span>
                </label>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Notification Settings */}
      {activeTab === 'notifications' && (
        <div className="card p-6">
          <div className="flex items-center mb-4">
            <BellIcon className="h-6 w-6 text-primary-600 mr-3" />
            <h2 className="text-lg font-semibold text-secondary-900 dark:text-white">
              Notifications
            </h2>
          </div>

          <div className="text-center py-8">
            <BellIcon className="h-16 w-16 text-secondary-400 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-secondary-900 dark:text-white mb-2">
              No Notifications Configured
            </h3>
            <p className="text-secondary-600 dark:text-secondary-400 mb-4">
              Set up notifications to stay informed about downloads and imports
            </p>
            <button className="btn-primary">
              <BellIcon className="h-5 w-5 mr-2" />
              Add Notification
            </button>
          </div>
        </div>
      )}

      {/* Application Info */}
      <div className="card p-6">
        <div className="flex items-center mb-4">
          <CogIcon className="h-6 w-6 text-primary-600 mr-3" />
          <h2 className="text-lg font-semibold text-secondary-900 dark:text-white">
            Application Information
          </h2>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-secondary-600 dark:text-secondary-400">Version:</span>
            <span className="ml-2 text-secondary-900 dark:text-white">
              {import.meta.env.VITE_APP_VERSION || '1.0.0'}
            </span>
          </div>
          <div>
            <span className="text-secondary-600 dark:text-secondary-400">Environment:</span>
            <span className="ml-2 text-secondary-900 dark:text-white">
              {import.meta.env.MODE}
            </span>
          </div>
          <div>
            <span className="text-secondary-600 dark:text-secondary-400">Build:</span>
            <span className="ml-2 text-secondary-900 dark:text-white">
              {import.meta.env.VITE_DEBUG === 'true' ? 'Development' : 'Production'}
            </span>
          </div>
          <div>
            <span className="text-secondary-600 dark:text-secondary-400">API URL:</span>
            <span className="ml-2 text-secondary-900 dark:text-white font-mono text-xs">
              {radarrApi.getConfig().baseUrl}
            </span>
          </div>
        </div>
      </div>

      {/* Development Info */}
      {import.meta.env.VITE_DEBUG === 'true' && (
        <div className="card p-6 border-warning-200 bg-warning-50 dark:bg-warning-900/20">
          <div className="flex items-center mb-3">
            <ExclamationTriangleIcon className="h-5 w-5 text-warning-600 mr-2" />
            <h3 className="text-lg font-medium text-warning-800 dark:text-warning-200">
              Development Mode
            </h3>
          </div>
          <p className="text-warning-700 dark:text-warning-300 text-sm">
            This application is running in development mode. Debug information and additional 
            logging are enabled. This should not be used in production.
          </p>
        </div>
      )}
    </div>
  );
};