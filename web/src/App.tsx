import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { ThemeProvider } from './contexts/ThemeContext';
import { UIProvider } from './contexts/UIContext';
import { ToastProvider } from './components/ui/Toast';
import { ConfirmDialogProvider } from './components/ui/ConfirmDialog';
import { Layout } from './components/layout/Layout';
import { Dashboard } from './pages/Dashboard';
import { Movies } from './pages/Movies';
import { AddMovie } from './pages/AddMovie';
import { Settings } from './pages/Settings';

function App() {
  return (
    <ThemeProvider>
      <UIProvider>
        <ToastProvider>
          <ConfirmDialogProvider>
            <Router>
              <Routes>
                <Route path="/" element={<Layout />}>
                  <Route index element={<Dashboard />} />
                  <Route path="movies" element={<Movies />} />
                  <Route path="add-movie" element={<AddMovie />} />
                  <Route path="settings" element={<Settings />} />
                  
                  {/* Placeholder routes for future implementation */}
                  <Route path="activity" element={<PlaceholderPage title="Activity" />} />
                  <Route path="search" element={<PlaceholderPage title="Search" />} />
                  <Route path="queue" element={<PlaceholderPage title="Queue" />} />
                </Route>
              </Routes>
            </Router>
          </ConfirmDialogProvider>
        </ToastProvider>
      </UIProvider>
    </ThemeProvider>
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
