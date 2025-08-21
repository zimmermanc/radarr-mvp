import React from 'react';
import { NavLink } from 'react-router-dom';
import {
  HomeIcon,
  FilmIcon,
  PlusIcon,
  CogIcon,
  ChartBarIcon,
  MagnifyingGlassIcon,
  ArchiveBoxIcon,
} from '@heroicons/react/24/outline';
import { useUI } from '../../contexts/UIContext';

const navigation = [
  { name: 'Dashboard', href: '/', icon: HomeIcon },
  { name: 'Movies', href: '/movies', icon: FilmIcon },
  { name: 'Add Movie', href: '/add-movie', icon: PlusIcon },
  { name: 'Activity', href: '/activity', icon: ChartBarIcon },
  { name: 'Search', href: '/search', icon: MagnifyingGlassIcon },
  { name: 'Queue', href: '/queue', icon: ArchiveBoxIcon },
  { name: 'Settings', href: '/settings', icon: CogIcon },
];

export const Sidebar: React.FC = () => {
  const { isMobile, setSidebarOpen } = useUI();

  const handleNavClick = () => {
    if (isMobile) {
      setSidebarOpen(false);
    }
  };

  return (
    <div className="flex w-64 flex-col md:static bg-white dark:bg-secondary-800 h-screen">
      <div className="flex flex-col flex-grow pt-5 overflow-y-auto border-r border-secondary-200 dark:border-secondary-700 safe-area-top">
        {/* Logo */}
        <div className="flex items-center flex-shrink-0 px-4">
          <div className="flex items-center">
            <div className="flex-shrink-0">
              <FilmIcon className="h-8 w-8 text-primary-600" />
            </div>
            <div className="ml-3">
              <h1 className="text-xl font-bold text-secondary-900 dark:text-white">
                Radarr
              </h1>
              <p className="text-xs text-secondary-500 dark:text-secondary-400">
                v{import.meta.env.VITE_APP_VERSION || '1.0.0'}
              </p>
            </div>
          </div>
        </div>

        {/* Navigation */}
        <nav className="mt-8 flex-1 px-4 space-y-1">
          {navigation.map((item) => (
            <NavLink
              key={item.name}
              to={item.href}
              onClick={handleNavClick}
              className={({ isActive }) =>
                `nav-link touch-target ${isActive ? 'nav-link-active' : 'nav-link-inactive'}`
              }
            >
              <item.icon
                className="mr-3 flex-shrink-0 h-5 w-5"
                aria-hidden="true"
              />
              <span className="text-sm md:text-base">{item.name}</span>
            </NavLink>
          ))}
        </nav>

        {/* Footer */}
        <div className="flex-shrink-0 p-4 border-t border-secondary-200 dark:border-secondary-700">
          <div className="flex items-center">
            <div className="flex-shrink-0">
              <div className="h-2 w-2 bg-success-500 rounded-full"></div>
            </div>
            <div className="ml-3">
              <p className="text-xs text-secondary-500 dark:text-secondary-400">
                API Connected
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};