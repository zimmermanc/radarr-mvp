import React from 'react';
import { ArrowRightOnRectangleIcon } from '@heroicons/react/24/outline';
import { useAuth } from '../../contexts/AuthContext';
import { useToast } from '../ui/Toast';

interface LogoutButtonProps {
  className?: string;
  variant?: 'button' | 'link';
  showIcon?: boolean;
  children?: React.ReactNode;
}

export function LogoutButton({ 
  className = '', 
  variant = 'button', 
  showIcon = true,
  children = 'Sign Out'
}: LogoutButtonProps) {
  const { logout } = useAuth();
  const { success } = useToast();

  const handleLogout = () => {
    logout();
    success('Logout', 'Successfully logged out');
  };

  const baseClasses = variant === 'button' 
    ? 'inline-flex items-center justify-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500 transition-colors'
    : 'inline-flex items-center text-sm text-red-600 hover:text-red-500 transition-colors';

  return (
    <button
      onClick={handleLogout}
      className={`${baseClasses} ${className}`}
    >
      {showIcon && <ArrowRightOnRectangleIcon className="h-4 w-4 mr-2" />}
      {children}
    </button>
  );
}