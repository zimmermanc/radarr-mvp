import React, { useState } from 'react';
import { useAuth } from '../../contexts/AuthContext';
import { useNavigate, useLocation } from 'react-router-dom';
import type { LoginCredentials } from '../../types/auth';

export function LoginPage() {
  const { login, isLoading } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  
  const [loginMethod, setLoginMethod] = useState<'credentials' | 'apikey'>('credentials');
  const [formData, setFormData] = useState<LoginCredentials>({
    username: '',
    password: '',
    apiKey: ''
  });
  const [error, setError] = useState<string>('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Get the redirect path from location state or default to dashboard
  const from = (location.state as any)?.from?.pathname || '/';

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setIsSubmitting(true);

    try {
      const credentials: LoginCredentials = loginMethod === 'apikey' 
        ? { apiKey: formData.apiKey }
        : { username: formData.username, password: formData.password };

      const result = await login(credentials);
      
      if (result.success) {
        // Redirect to intended page or dashboard
        navigate(from, { replace: true });
      } else {
        setError(result.message || 'Login failed');
      }
    } catch (err) {
      setError('An unexpected error occurred. Please try again.');
      console.error('Login error:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleInputChange = (field: keyof LoginCredentials) => (
    e: React.ChangeEvent<HTMLInputElement>
  ) => {
    setFormData(prev => ({
      ...prev,
      [field]: e.target.value
    }));
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-primary-50 dark:bg-primary-900 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-accent-500"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-primary-50 to-secondary-50 dark:from-primary-900 dark:to-secondary-900 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        {/* Logo and Header */}
        <div className="text-center mb-8">
          <div className="mx-auto w-16 h-16 bg-accent-500 rounded-xl flex items-center justify-center mb-4">
            <svg className="w-8 h-8 text-white" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M3 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1V4zm0 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1V8zm0 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1v-2z" clipRule="evenodd" />
            </svg>
          </div>
          <h1 className="text-3xl font-bold text-secondary-900 dark:text-white">
            Radarr
          </h1>
          <p className="text-secondary-600 dark:text-secondary-400 mt-2">
            Sign in to continue to your dashboard
          </p>
        </div>

        {/* Login Form */}
        <div className="bg-white dark:bg-secondary-800 rounded-lg shadow-lg p-6">
          {/* Login Method Toggle */}
          <div className="flex rounded-lg p-1 bg-secondary-100 dark:bg-secondary-700 mb-6">
            <button
              type="button"
              onClick={() => setLoginMethod('credentials')}
              className={`flex-1 py-2 px-4 rounded-md text-sm font-medium transition-colors ${
                loginMethod === 'credentials'
                  ? 'bg-white dark:bg-secondary-600 text-secondary-900 dark:text-white shadow-sm'
                  : 'text-secondary-600 dark:text-secondary-400 hover:text-secondary-900 dark:hover:text-white'
              }`}
            >
              Username & Password
            </button>
            <button
              type="button"
              onClick={() => setLoginMethod('apikey')}
              className={`flex-1 py-2 px-4 rounded-md text-sm font-medium transition-colors ${
                loginMethod === 'apikey'
                  ? 'bg-white dark:bg-secondary-600 text-secondary-900 dark:text-white shadow-sm'
                  : 'text-secondary-600 dark:text-secondary-400 hover:text-secondary-900 dark:hover:text-white'
              }`}
            >
              API Key
            </button>
          </div>

          {/* Error Message */}
          {error && (
            <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
              <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
            </div>
          )}

          {/* Form */}
          <form onSubmit={handleSubmit} className="space-y-4">
            {loginMethod === 'credentials' ? (
              <>
                <div>
                  <label htmlFor="username" className="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-2">
                    Username
                  </label>
                  <input
                    id="username"
                    type="text"
                    value={formData.username}
                    onChange={handleInputChange('username')}
                    className="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-md 
                             bg-white dark:bg-secondary-700 text-secondary-900 dark:text-white
                             focus:ring-2 focus:ring-accent-500 focus:border-accent-500 
                             placeholder-secondary-400 dark:placeholder-secondary-500"
                    placeholder="Enter your username"
                    required
                    disabled={isSubmitting}
                    autoComplete="username"
                  />
                </div>
                <div>
                  <label htmlFor="password" className="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-2">
                    Password
                  </label>
                  <input
                    id="password"
                    type="password"
                    value={formData.password}
                    onChange={handleInputChange('password')}
                    className="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-md 
                             bg-white dark:bg-secondary-700 text-secondary-900 dark:text-white
                             focus:ring-2 focus:ring-accent-500 focus:border-accent-500 
                             placeholder-secondary-400 dark:placeholder-secondary-500"
                    placeholder="Enter your password"
                    required
                    disabled={isSubmitting}
                    autoComplete="current-password"
                  />
                </div>
              </>
            ) : (
              <div>
                <label htmlFor="apiKey" className="block text-sm font-medium text-secondary-700 dark:text-secondary-300 mb-2">
                  API Key
                </label>
                <input
                  id="apiKey"
                  type="password"
                  value={formData.apiKey}
                  onChange={handleInputChange('apiKey')}
                  className="w-full px-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-md 
                           bg-white dark:bg-secondary-700 text-secondary-900 dark:text-white
                           focus:ring-2 focus:ring-accent-500 focus:border-accent-500 
                           placeholder-secondary-400 dark:placeholder-secondary-500"
                  placeholder="Enter your API key"
                  required
                  disabled={isSubmitting}
                />
                <p className="mt-2 text-xs text-secondary-500 dark:text-secondary-400">
                  You can find your API key in the Radarr settings under General.
                </p>
              </div>
            )}

            {/* Submit Button */}
            <button
              type="submit"
              disabled={isSubmitting}
              className="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm 
                       text-sm font-medium text-white bg-accent-600 hover:bg-accent-700 
                       focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-accent-500
                       disabled:opacity-50 disabled:cursor-not-allowed
                       dark:focus:ring-offset-secondary-800 transition-colors"
            >
              {isSubmitting ? (
                <span className="flex items-center">
                  <svg className="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                  </svg>
                  Signing in...
                </span>
              ) : (
                'Sign In'
              )}
            </button>
          </form>

          {/* Default Credentials Info */}
          {loginMethod === 'credentials' && (
            <div className="mt-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md">
              <p className="text-sm text-blue-600 dark:text-blue-400 font-medium mb-1">
                Default Credentials:
              </p>
              <p className="text-xs text-blue-600 dark:text-blue-400">
                Username: <code className="bg-blue-100 dark:bg-blue-800 px-1 rounded">admin</code> | 
                Password: <code className="bg-blue-100 dark:bg-blue-800 px-1 rounded ml-1">admin</code>
              </p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="mt-8 text-center">
          <p className="text-xs text-secondary-500 dark:text-secondary-400">
            Radarr Movie Collection Manager
          </p>
        </div>
      </div>
    </div>
  );
}