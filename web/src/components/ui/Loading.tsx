import React from 'react';

// Spinner Component
interface SpinnerProps {
  size?: 'sm' | 'md' | 'lg' | 'xl';
  color?: 'primary' | 'secondary' | 'white';
  className?: string;
}

export const Spinner: React.FC<SpinnerProps> = ({ 
  size = 'md', 
  color = 'primary',
  className = ''
}) => {
  const sizeClasses = {
    sm: 'h-4 w-4',
    md: 'h-6 w-6',
    lg: 'h-8 w-8',
    xl: 'h-12 w-12',
  };

  const colorClasses = {
    primary: 'text-primary-600',
    secondary: 'text-secondary-400',
    white: 'text-white',
  };

  return (
    <svg
      className={`animate-spin ${sizeClasses[size]} ${colorClasses[color]} ${className}`}
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
      aria-label="Loading"
    >
      <circle
        className="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        strokeWidth="4"
      />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      />
    </svg>
  );
};

// Loading Button Component
interface LoadingButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  loading?: boolean;
  loadingText?: string;
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  children: React.ReactNode;
}

export const LoadingButton: React.FC<LoadingButtonProps> = ({
  loading = false,
  loadingText,
  variant = 'primary',
  size = 'md',
  children,
  disabled,
  className = '',
  ...props
}) => {
  const baseClasses = 'inline-flex items-center justify-center font-medium rounded-lg transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed';
  
  const variantClasses = {
    primary: 'bg-primary-600 hover:bg-primary-700 text-white focus:ring-primary-500 disabled:hover:bg-primary-600',
    secondary: 'bg-secondary-200 hover:bg-secondary-300 text-secondary-700 focus:ring-secondary-500 dark:bg-secondary-700 dark:text-secondary-200 dark:hover:bg-secondary-600',
    ghost: 'hover:bg-secondary-100 text-secondary-700 focus:ring-secondary-500 dark:text-secondary-300 dark:hover:bg-secondary-700',
    danger: 'bg-error-600 hover:bg-error-700 text-white focus:ring-error-500 disabled:hover:bg-error-600',
  };

  const sizeClasses = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-sm',
    lg: 'px-6 py-3 text-base',
  };

  const spinnerSize = {
    sm: 'sm' as const,
    md: 'sm' as const,
    lg: 'md' as const,
  };

  return (
    <button
      {...props}
      disabled={disabled || loading}
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
    >
      {loading && (
        <Spinner 
          size={spinnerSize[size]} 
          color={variant === 'secondary' || variant === 'ghost' ? 'secondary' : 'white'}
          className="mr-2" 
        />
      )}
      {loading && loadingText ? loadingText : children}
    </button>
  );
};

// Page Loading Component
interface PageLoadingProps {
  message?: string;
  size?: 'sm' | 'md' | 'lg';
}

export const PageLoading: React.FC<PageLoadingProps> = ({ 
  message = 'Loading...', 
  size = 'lg' 
}) => {
  return (
    <div className="flex flex-col items-center justify-center py-12 space-y-4">
      <Spinner size={size} />
      <div className="text-secondary-600 dark:text-secondary-400 text-sm font-medium">
        {message}
      </div>
    </div>
  );
};

// Skeleton Loading Components
interface SkeletonProps {
  className?: string;
  variant?: 'text' | 'circular' | 'rectangular';
  width?: string | number;
  height?: string | number;
  lines?: number;
}

export const Skeleton: React.FC<SkeletonProps> = ({
  className = '',
  variant = 'rectangular',
  width,
  height,
  lines = 1,
}) => {
  const baseClasses = 'animate-pulse bg-secondary-200 dark:bg-secondary-700';
  
  const variantClasses = {
    text: 'rounded h-4',
    circular: 'rounded-full',
    rectangular: 'rounded',
  };

  if (variant === 'text' && lines > 1) {
    return (
      <div className={`space-y-2 ${className}`}>
        {Array.from({ length: lines }).map((_, index) => (
          <div
            key={index}
            className={`${baseClasses} ${variantClasses.text}`}
            style={{
              width: index === lines - 1 ? '75%' : '100%',
              height: height || '1rem',
            }}
          />
        ))}
      </div>
    );
  }

  return (
    <div
      className={`${baseClasses} ${variantClasses[variant]} ${className}`}
      style={{
        width: width || (variant === 'circular' ? '40px' : '100%'),
        height: height || (variant === 'circular' ? '40px' : '20px'),
      }}
    />
  );
};

// Movie Card Skeleton
export const MovieCardSkeleton: React.FC = () => {
  return (
    <div className="card p-4 space-y-4">
      <Skeleton variant="rectangular" height="200px" className="rounded-lg" />
      <div className="space-y-2">
        <Skeleton variant="text" width="80%" />
        <Skeleton variant="text" width="60%" />
        <div className="flex items-center space-x-2 pt-2">
          <Skeleton variant="rectangular" width="60px" height="24px" className="rounded-full" />
          <Skeleton variant="rectangular" width="80px" height="24px" className="rounded-full" />
        </div>
      </div>
    </div>
  );
};

// Table Row Skeleton
interface TableRowSkeletonProps {
  columns: number;
  rows?: number;
}

export const TableRowSkeleton: React.FC<TableRowSkeletonProps> = ({ 
  columns, 
  rows = 1 
}) => {
  return (
    <>
      {Array.from({ length: rows }).map((_, rowIndex) => (
        <tr key={rowIndex} className="border-b border-secondary-200 dark:border-secondary-700">
          {Array.from({ length: columns }).map((_, colIndex) => (
            <td key={colIndex} className="px-6 py-4">
              <Skeleton variant="text" />
            </td>
          ))}
        </tr>
      ))}
    </>
  );
};

// Dashboard Stats Skeleton
export const DashboardStatsSkeleton: React.FC = () => {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      {Array.from({ length: 4 }).map((_, index) => (
        <div key={index} className="card p-6">
          <div className="flex items-center justify-between">
            <div className="space-y-2">
              <Skeleton variant="text" width="60px" />
              <Skeleton variant="text" width="40px" height="28px" />
            </div>
            <Skeleton variant="circular" width="48px" height="48px" />
          </div>
        </div>
      ))}
    </div>
  );
};

// Loading Overlay Component
interface LoadingOverlayProps {
  loading: boolean;
  message?: string;
  children: React.ReactNode;
}

export const LoadingOverlay: React.FC<LoadingOverlayProps> = ({
  loading,
  message = 'Loading...',
  children,
}) => {
  return (
    <div className="relative">
      {children}
      {loading && (
        <div className="absolute inset-0 bg-white/75 dark:bg-secondary-900/75 backdrop-blur-sm flex items-center justify-center z-10 rounded-lg">
          <div className="flex flex-col items-center space-y-3">
            <Spinner size="lg" />
            <div className="text-secondary-600 dark:text-secondary-400 text-sm font-medium">
              {message}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

// Inline Loading Component (for buttons and small spaces)
interface InlineLoadingProps {
  message?: string;
  size?: 'sm' | 'md';
}

export const InlineLoading: React.FC<InlineLoadingProps> = ({ 
  message, 
  size = 'sm' 
}) => {
  return (
    <div className="flex items-center space-x-2">
      <Spinner size={size} color="secondary" />
      {message && (
        <span className="text-secondary-600 dark:text-secondary-400 text-sm">
          {message}
        </span>
      )}
    </div>
  );
};