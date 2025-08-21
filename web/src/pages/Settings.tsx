import React, { useState, useEffect } from 'react';
import { 
  CogIcon,
  ServerIcon,
  KeyIcon,
  CheckCircleIcon,
  ExclamationTriangleIcon,
  ArrowPathIcon 
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

      {/* API Configuration */}
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