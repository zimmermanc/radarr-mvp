import React, { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { ThemeProvider } from './contexts/ThemeContext';
import { UIProvider } from './contexts/UIContext';
import { AuthProvider } from './contexts/AuthContext';
import { ToastProvider } from './components/ui/Toast';
import { ConfirmDialogProvider } from './components/ui/ConfirmDialog';
import { WebSocketProvider } from './contexts/WebSocketContext';
import { Layout } from './components/layout/Layout';
import { ProtectedRoute } from './components/auth/ProtectedRoute';
import { LoginPage } from './components/auth/LoginPage';
import { Dashboard } from './pages/Dashboard';
import { Movies } from './pages/Movies';
import { AddMovie } from './pages/AddMovie';
import { Settings } from './pages/Settings';
import { Queue } from './pages/Queue';
import Streaming from './pages/Streaming';
import { initStreamingApi } from './lib/streamingApi';
import { ErrorBoundary } from './components/ErrorBoundary';

function App() {
  // v1.0.3 - Fixed critical JavaScript iteration errors in production
  // Initialize streaming API globally
  useEffect(() => {
    const baseUrl = import.meta.env.VITE_API_URL || window.location.origin;
    const apiKey = import.meta.env.VITE_API_KEY || 'YOUR_API_KEY_HERE';
    initStreamingApi(baseUrl, apiKey);
  }, []);

  return (
    <ErrorBoundary>
      <ThemeProvider>
      <UIProvider>
        <ToastProvider>
          <ConfirmDialogProvider>
            <AuthProvider>
              <WebSocketProvider>
                <Router>
                  <Routes>
                    {/* Public login route */}
                    <Route path="/login" element={<LoginPage />} />
                    
                    {/* Protected routes */}
                    <Route path="/" element={
                      <ProtectedRoute>
                        <Layout />
                      </ProtectedRoute>
                    }>
                      <Route index element={<Dashboard />} />
                      <Route path="movies" element={<Movies />} />
                      <Route path="add-movie" element={<AddMovie />} />
                      <Route path="settings" element={<Settings />} />
                      <Route path="queue" element={<Queue />} />
                      <Route path="streaming" element={<Streaming />} />
                      
                      {/* Placeholder routes for future implementation */}
                      <Route path="activity" element={<PlaceholderPage title="Activity" />} />
                      <Route path="search" element={<PlaceholderPage title="Search" />} />
                    </Route>
                  </Routes>
                </Router>
              </WebSocketProvider>
            </AuthProvider>
          </ConfirmDialogProvider>
        </ToastProvider>
      </UIProvider>
    </ThemeProvider>
    </ErrorBoundary>
  );
}

// Placeholder component for unimplemented pages
const PlaceholderPage: React.FC<{ title: string }> = ({ title }) => (
  <div className="p-6">
    <div className="card p-8 text-center">
      <h1 className="text-2xl font-bold text-secondary-900 dark:text-white mb-4">
        {title}
      </h1>
      <p className="text-secondary-600 dark:text-secondary-400">
        This page is coming soon! The {title.toLowerCase()} functionality will be implemented in a future update.
      </p>
    </div>
  </div>
);

export default App;
