import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { 
  Bars3Icon, 
  BellIcon, 
  MagnifyingGlassIcon,
  Cog6ToothIcon,
  ArrowRightOnRectangleIcon,
  UserCircleIcon
} from '@heroicons/react/24/outline';
import { useUI } from '../../contexts/UIContext';
import { useAuth } from '../../contexts/AuthContext';
import { useWebSocket } from '../../contexts/WebSocketContext';
import { ThemeToggle } from '../ui/ThemeToggle';
import { useToast } from '../ui/Toast';
import { useClickOutside } from '../../hooks/useClickOutside';

export const Header: React.FC = () => {
  const navigate = useNavigate();
  const [searchQuery, setSearchQuery] = useState('');
  const [showUserMenu, setShowUserMenu] = useState(false);
  const { 
    toggleSidebar, 
    notificationCount, 
    clearNotifications,
    toggleGlobalSearch 
  } = useUI();
  const { authState, logout } = useAuth();
  const { isConnected, connectionError } = useWebSocket();
  const { info, success } = useToast();
  
  // Click outside to close user menu
  const userMenuRef = useClickOutside<HTMLDivElement>(() => setShowUserMenu(false));

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      // Navigate to Add Movie page with search query
      navigate(`/add-movie?search=${encodeURIComponent(searchQuery.trim())}`);
      setSearchQuery(''); // Clear search after navigation
    }
  };

  const handleNotificationClick = () => {
    if (notificationCount > 0) {
      clearNotifications();
      info('Notifications', 'All notifications cleared');
    }
  };

  const handleLogout = () => {
    logout();
    success('Logout', 'Successfully logged out');
    setShowUserMenu(false);
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

            {/* User menu */}
            <div ref={userMenuRef} className="relative pl-3 border-l border-secondary-200 dark:border-secondary-600">
              <button
                onClick={() => setShowUserMenu(!showUserMenu)}
                className="flex items-center space-x-2 p-2 text-secondary-400 hover:text-secondary-500 dark:hover:text-secondary-300 hover:bg-secondary-100 dark:hover:bg-secondary-700 rounded-lg transition-colors duration-200 touch-target"
                aria-label="User menu"
              >
                <UserCircleIcon className="h-5 w-5" />
                <span className="text-sm text-secondary-600 dark:text-secondary-400 hidden sm:inline">
                  {authState.user?.username || 'User'}
                </span>
              </button>

              {/* User dropdown menu */}
              {showUserMenu && (
                <div className="absolute right-0 mt-2 w-48 bg-white dark:bg-secondary-800 rounded-md shadow-lg py-1 z-50 border border-secondary-200 dark:border-secondary-700">
                  <div className="px-4 py-2 border-b border-secondary-200 dark:border-secondary-700">
                    <p className="text-sm text-secondary-900 dark:text-white font-medium">
                      {authState.user?.username}
                    </p>
                    <p className="text-xs text-secondary-500 dark:text-secondary-400">
                      API Key: {authState.apiKey ? '••••••••' : 'Not set'}
                    </p>
                  </div>
                  <button
                    onClick={handleLogout}
                    className="w-full text-left px-4 py-2 text-sm text-secondary-700 dark:text-secondary-300 hover:bg-secondary-100 dark:hover:bg-secondary-700 flex items-center space-x-2"
                  >
                    <ArrowRightOnRectangleIcon className="h-4 w-4" />
                    <span>Sign out</span>
                  </button>
                </div>
              )}
            </div>

            {/* WebSocket connection status */}
            <div className="flex items-center space-x-2 pl-3 border-l border-secondary-200 dark:border-secondary-600">
              <div className="flex items-center space-x-1" title={connectionError || (isConnected ? 'Real-time updates connected' : 'Real-time updates disconnected')}>
                <div 
                  className={`h-2 w-2 rounded-full ${
                    isConnected 
                      ? 'bg-success-500 animate-pulse' 
                      : connectionError 
                        ? 'bg-error-500' 
                        : 'bg-warning-500'
                  }`}
                ></div>
                <span className="text-xs text-secondary-500 dark:text-secondary-400 hidden sm:inline">
                  {isConnected ? 'Live' : connectionError ? 'Error' : 'Offline'}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </header>
  );
};