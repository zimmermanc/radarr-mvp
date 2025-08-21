import React, { useState } from 'react';
import { 
  SunIcon, 
  MoonIcon, 
  ComputerDesktopIcon,
  ChevronDownIcon 
} from '@heroicons/react/24/outline';
import { useTheme, type Theme } from '../../contexts/ThemeContext';

interface ThemeToggleProps {
  variant?: 'button' | 'dropdown';
  showLabel?: boolean;
  className?: string;
}

export const ThemeToggle: React.FC<ThemeToggleProps> = ({ 
  variant = 'button',
  showLabel = false,
  className = ''
}) => {
  const { theme, actualTheme, setTheme, toggleTheme } = useTheme();
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);

  const themeOptions: { value: Theme; label: string; icon: React.ReactNode }[] = [
    {
      value: 'light',
      label: 'Light',
      icon: <SunIcon className="h-4 w-4" />,
    },
    {
      value: 'dark',
      label: 'Dark',
      icon: <MoonIcon className="h-4 w-4" />,
    },
    {
      value: 'system',
      label: 'System',
      icon: <ComputerDesktopIcon className="h-4 w-4" />,
    },
  ];

  const currentOption = themeOptions.find(option => option.value === theme);

  if (variant === 'button') {
    return (
      <button
        onClick={toggleTheme}
        className={`p-2 rounded-lg text-secondary-600 dark:text-secondary-400 hover:bg-secondary-100 dark:hover:bg-secondary-700 transition-colors duration-200 ${className}`}
        aria-label={`Switch to ${actualTheme === 'dark' ? 'light' : 'dark'} mode`}
        title={`Currently in ${actualTheme} mode. Click to toggle.`}
      >
        {actualTheme === 'dark' ? (
          <SunIcon className="h-5 w-5" />
        ) : (
          <MoonIcon className="h-5 w-5" />
        )}
        {showLabel && (
          <span className="ml-2 text-sm font-medium">
            {actualTheme === 'dark' ? 'Light mode' : 'Dark mode'}
          </span>
        )}
      </button>
    );
  }

  return (
    <div className={`relative ${className}`}>
      <button
        onClick={() => setIsDropdownOpen(!isDropdownOpen)}
        className="flex items-center space-x-2 p-2 rounded-lg text-secondary-600 dark:text-secondary-400 hover:bg-secondary-100 dark:hover:bg-secondary-700 transition-colors duration-200"
        aria-label="Theme options"
        aria-expanded={isDropdownOpen}
        aria-haspopup="true"
      >
        {currentOption?.icon}
        {showLabel && (
          <span className="text-sm font-medium">{currentOption?.label}</span>
        )}
        <ChevronDownIcon 
          className={`h-4 w-4 transition-transform duration-200 ${
            isDropdownOpen ? 'rotate-180' : ''
          }`} 
        />
      </button>

      {isDropdownOpen && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 z-10"
            onClick={() => setIsDropdownOpen(false)}
          />
          
          {/* Dropdown */}
          <div className="absolute right-0 mt-2 w-36 bg-white dark:bg-secondary-800 rounded-lg shadow-lg border border-secondary-200 dark:border-secondary-700 z-20 py-1">
            {themeOptions.map((option) => (
              <button
                key={option.value}
                onClick={() => {
                  setTheme(option.value);
                  setIsDropdownOpen(false);
                }}
                className={`w-full flex items-center space-x-2 px-3 py-2 text-sm text-left hover:bg-secondary-50 dark:hover:bg-secondary-700 transition-colors ${
                  theme === option.value
                    ? 'text-primary-600 dark:text-primary-400 bg-primary-50 dark:bg-primary-900/20'
                    : 'text-secondary-700 dark:text-secondary-300'
                }`}
              >
                {option.icon}
                <span>{option.label}</span>
                {theme === option.value && (
                  <div className="ml-auto w-2 h-2 bg-primary-600 dark:bg-primary-400 rounded-full" />
                )}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
};

// Compact theme toggle for mobile
export const CompactThemeToggle: React.FC<{ className?: string }> = ({ className }) => {
  const { actualTheme, toggleTheme } = useTheme();

  return (
    <button
      onClick={toggleTheme}
      className={`p-2 rounded-full bg-secondary-100 dark:bg-secondary-700 text-secondary-600 dark:text-secondary-400 hover:bg-secondary-200 dark:hover:bg-secondary-600 transition-all duration-200 ${className}`}
      aria-label={`Switch to ${actualTheme === 'dark' ? 'light' : 'dark'} mode`}
    >
      <div className="relative h-5 w-5">
        <SunIcon 
          className={`absolute inset-0 h-5 w-5 transition-all duration-300 ${
            actualTheme === 'dark' 
              ? 'opacity-0 rotate-90 scale-0' 
              : 'opacity-100 rotate-0 scale-100'
          }`} 
        />
        <MoonIcon 
          className={`absolute inset-0 h-5 w-5 transition-all duration-300 ${
            actualTheme === 'dark' 
              ? 'opacity-100 rotate-0 scale-100' 
              : 'opacity-0 -rotate-90 scale-0'
          }`} 
        />
      </div>
    </button>
  );
};