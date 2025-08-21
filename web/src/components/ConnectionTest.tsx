import React, { useState, useEffect } from 'react';
import { radarrApi } from '../lib/api';

const ConnectionTest: React.FC = () => {
  const [status, setStatus] = useState<'idle' | 'testing' | 'success' | 'error'>('idle');
  const [message, setMessage] = useState('');
  const [movies, setMovies] = useState<any[]>([]);

  const testConnection = async () => {
    setStatus('testing');
    setMessage('Testing API connection...');
    
    try {
      // Test health endpoint
      const healthResponse = await radarrApi.getHealth();
      if (!healthResponse.success) {
        throw new Error('Health check failed');
      }

      // Test movies endpoint
      const moviesResponse = await radarrApi.getMovies();
      if (!moviesResponse.success) {
        throw new Error('Movies endpoint failed');
      }

      setMovies(moviesResponse.data.movies);
      setStatus('success');
      setMessage(`Successfully connected! Found ${moviesResponse.data.movies.length} movies.`);
    } catch (error) {
      setStatus('error');
      setMessage(`Connection failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  useEffect(() => {
    testConnection();
  }, []);

  return (
    <div className="p-6 bg-white rounded-lg shadow-md">
      <h2 className="text-2xl font-bold mb-4">API Connection Test</h2>
      
      <div className="mb-4">
        <div className={`p-3 rounded-md ${
          status === 'testing' ? 'bg-yellow-100 text-yellow-800' :
          status === 'success' ? 'bg-green-100 text-green-800' :
          status === 'error' ? 'bg-red-100 text-red-800' :
          'bg-gray-100 text-gray-800'
        }`}>
          {status === 'testing' && '⏳ '}
          {status === 'success' && '✅ '}
          {status === 'error' && '❌ '}
          {message}
        </div>
      </div>

      <button 
        onClick={testConnection}
        disabled={status === 'testing'}
        className="px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed mb-4"
      >
        {status === 'testing' ? 'Testing...' : 'Test Connection'}
      </button>

      {status === 'success' && movies.length > 0 && (
        <div className="mt-4">
          <h3 className="text-lg font-semibold mb-2">Movies Found:</h3>
          <div className="space-y-2">
            {movies.map((movie) => (
              <div key={movie.id} className="p-2 bg-gray-50 rounded border">
                <div className="font-medium">{movie.title} ({movie.year})</div>
                <div className="text-sm text-gray-600">
                  Status: {movie.status} | Monitored: {movie.monitored ? 'Yes' : 'No'}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="mt-4 text-sm text-gray-500">
        <div>Base URL: {radarrApi.getConfig().baseUrl || '(relative URLs via proxy)'}</div>
        <div>Current Location: {window.location.href}</div>
        <div>Environment: {import.meta.env.DEV ? 'Development' : 'Production'}</div>
      </div>
    </div>
  );
};

export default ConnectionTest;