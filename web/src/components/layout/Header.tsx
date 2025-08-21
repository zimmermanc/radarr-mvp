import React, { useState } from 'react';
import { 
  Bars3Icon, 
  BellIcon, 
  MagnifyingGlassIcon,
  Cog6ToothIcon 
} from '@heroicons/react/24/outline';
import { useUI } from '../../contexts/UIContext';
import { ThemeToggle } from '../ui/ThemeToggle';
import { useToast } from '../ui/Toast';

export const Header: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');
  const { 
    toggleSidebar, 
    isMobile, 
    notificationCount, 
    clearNotifications,
    toggleGlobalSearch 
  } = useUI();
  const { info } = useToast();

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      info('Search', `Searching for "${searchQuery}"...`);
      // TODO: Implement actual search functionality
      console.log('Search query:', searchQuery);
    }
  };

  const handleNotificationClick = () => {
    if (notificationCount > 0) {
      clearNotifications();
      info('Notifications', 'All notifications cleared');
    }
  };

  return (
    <header className="bg-white dark:bg-secondary-800 shadow-sm border-b border-secondary-200 dark:border-secondary-700">
      <div className="px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          {/* Mobile menu button */}
          <div className="md:hidden">
            <button
              type="button"
              onClick={toggleSidebar}
              className="inline-flex items-center justify-center p-2 rounded-md text-secondary-400 hover:text-secondary-500 hover:bg-secondary-100 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-primary-500 transition-colors"
              aria-label="Toggle sidebar"
            >
              <Bars3Icon className="h-6 w-6" aria-hidden="true" />
            </button>
          </div>

          {/* Search bar */}
          <div className="flex-1 max-w-lg mx-auto md:mx-0 hidden sm:block">
            <form onSubmit={handleSearch} className="relative">
              <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <MagnifyingGlassIcon className="h-5 w-5 text-secondary-400" />
              </div>
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="block w-full pl-10 pr-3 py-2 border border-secondary-300 dark:border-secondary-600 rounded-lg leading-5 bg-white dark:bg-secondary-700 placeholder-secondary-500 dark:placeholder-secondary-400 text-secondary-900 dark:text-white focus:outline-none focus:placeholder-secondary-400 focus:ring-1 focus:ring-primary-500 focus:border-primary-500 transition-all"
                placeholder="Search movies..."
              />
            </form>
          </div>

          {/* Mobile search button */}
          <div className="sm:hidden">
            <button
              onClick={toggleGlobalSearch}
              className="p-2 text-secondary-400 hover:text-secondary-500 dark:hover:text-secondary-300 hover:bg-secondary-100 dark:hover:bg-secondary-700 rounded-lg transition-colors touch-target"
              aria-label="Search"
            >
              <MagnifyingGlassIcon className="h-5 w-5" />
            </button>
          </div>

          {/* Right side actions */}
          <div className="flex items-center space-x-1 sm:space-x-2">
            {/* Theme toggle */}
            <ThemeToggle variant="button" className="touch-target" />

            {/* Notifications */}
            <button 
              onClick={handleNotificationClick}
              className="p-2 text-secondary-400 hover:text-secondary-500 dark:hover:text-secondary-300 hover:bg-secondary-100 dark:hover:bg-secondary-700 rounded-lg transition-colors duration-200 relative touch-target"
              title={`${notificationCount} notifications`}
              aria-label={`${notificationCount} notifications`}
            >
              <BellIcon className="h-5 w-5" />
              {/* Notification badge */}
              {notificationCount > 0 && (
                <span className="absolute -top-1 -right-1 h-5 w-5 bg-error-500 text-white text-xs rounded-full flex items-center justify-center font-medium">
                  {notificationCount > 9 ? '9+' : notificationCount}
                </span>
              )}
            </button>

            {/* Settings - Hidden on mobile, shown in sidebar instead */}
            <button 
              className="hidden sm:flex p-2 text-secondary-400 hover:text-secondary-500 dark:hover:text-secondary-300 hover:bg-secondary-100 dark:hover:bg-secondary-700 rounded-lg transition-colors duration-200 touch-target"
              aria-label="Settings"
            >
              <Cog6ToothIcon className="h-5 w-5" />
            </button>

            {/* System status */}
            <div className="flex items-center space-x-2 pl-3 border-l border-secondary-200 dark:border-secondary-600">
              <div className="flex items-center space-x-1">
                <div className="h-2 w-2 bg-success-500 rounded-full animate-pulse"></div>
                <span className="text-xs text-secondary-500 dark:text-secondary-400 hidden sm:inline">
                  Healthy
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </header>
  );
};