import React, { createContext, useContext, useState, useCallback } from 'react';
import { 
  ExclamationTriangleIcon, 
  TrashIcon, 
  XMarkIcon,
  InformationCircleIcon
} from '@heroicons/react/24/outline';
import { LoadingButton } from './Loading';

export type ConfirmDialogType = 'danger' | 'warning' | 'info';

export interface ConfirmDialogOptions {
  type?: ConfirmDialogType;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  icon?: React.ReactNode;
  onConfirm: () => void | Promise<void>;
  onCancel?: () => void;
  destructive?: boolean;
}

interface ConfirmDialogContextType {
  confirm: (options: ConfirmDialogOptions) => void;
  isOpen: boolean;
}

const ConfirmDialogContext = createContext<ConfirmDialogContextType | undefined>(undefined);

export const useConfirmDialog = () => {
  const context = useContext(ConfirmDialogContext);
  if (!context) {
    throw new Error('useConfirmDialog must be used within a ConfirmDialogProvider');
  }
  return context;
};

interface ConfirmDialogProviderProps {
  children: React.ReactNode;
}

export const ConfirmDialogProvider: React.FC<ConfirmDialogProviderProps> = ({ children }) => {
  const [dialogState, setDialogState] = useState<{
    isOpen: boolean;
    options: ConfirmDialogOptions | null;
  }>({
    isOpen: false,
    options: null,
  });

  const confirm = useCallback((options: ConfirmDialogOptions) => {
    setDialogState({
      isOpen: true,
      options: {
        type: 'danger',
        confirmText: 'Confirm',
        cancelText: 'Cancel',
        ...options,
      },
    });
  }, []);

  const closeDialog = useCallback(() => {
    setDialogState({
      isOpen: false,
      options: null,
    });
  }, []);

  const value: ConfirmDialogContextType = {
    confirm,
    isOpen: dialogState.isOpen,
  };

  return (
    <ConfirmDialogContext.Provider value={value}>
      {children}
      {dialogState.isOpen && dialogState.options && (
        <ConfirmDialogModal
          options={dialogState.options}
          onClose={closeDialog}
        />
      )}
    </ConfirmDialogContext.Provider>
  );
};

interface ConfirmDialogModalProps {
  options: ConfirmDialogOptions;
  onClose: () => void;
}

const ConfirmDialogModal: React.FC<ConfirmDialogModalProps> = ({ options, onClose }) => {
  const [isLoading, setIsLoading] = useState(false);

  const handleConfirm = async () => {
    try {
      setIsLoading(true);
      await options.onConfirm();
      onClose();
    } catch (error) {
      console.error('Confirmation action failed:', error);
      // Don't close the dialog on error - let the user try again or cancel
    } finally {
      setIsLoading(false);
    }
  };

  const handleCancel = () => {
    if (options.onCancel) {
      options.onCancel();
    }
    onClose();
  };

  const getDialogStyles = () => {
    const styles = {
      danger: {
        iconColor: 'text-error-600 dark:text-error-400',
        iconBg: 'bg-error-100 dark:bg-error-900/20',
        buttonClass: 'danger' as const,
      },
      warning: {
        iconColor: 'text-warning-600 dark:text-warning-400',
        iconBg: 'bg-warning-100 dark:bg-warning-900/20',
        buttonClass: 'primary' as const,
      },
      info: {
        iconColor: 'text-primary-600 dark:text-primary-400',
        iconBg: 'bg-primary-100 dark:bg-primary-900/20',
        buttonClass: 'primary' as const,
      },
    };

    return styles[options.type || 'danger'];
  };

  const getDefaultIcon = () => {
    const iconProps = { className: 'h-6 w-6' };
    
    switch (options.type) {
      case 'warning':
        return <ExclamationTriangleIcon {...iconProps} />;
      case 'info':
        return <InformationCircleIcon {...iconProps} />;
      case 'danger':
      default:
        return options.destructive ? <TrashIcon {...iconProps} /> : <ExclamationTriangleIcon {...iconProps} />;
    }
  };

  const dialogStyles = getDialogStyles();

  return (
    <div 
      className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 z-50"
      onClick={handleCancel}
    >
      <div 
        className="bg-white dark:bg-secondary-800 rounded-xl shadow-xl max-w-md w-full p-6 animate-slide-up"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
        aria-describedby="dialog-description"
      >
        {/* Header */}
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-center space-x-3">
            <div className={`p-2 rounded-full ${dialogStyles.iconBg} ${dialogStyles.iconColor}`}>
              {options.icon || getDefaultIcon()}
            </div>
            <h3 
              id="dialog-title"
              className="text-lg font-semibold text-secondary-900 dark:text-white"
            >
              {options.title}
            </h3>
          </div>
          
          <button
            onClick={handleCancel}
            disabled={isLoading}
            className="p-1 text-secondary-400 hover:text-secondary-600 dark:hover:text-secondary-300 rounded-md hover:bg-secondary-100 dark:hover:bg-secondary-700 transition-colors disabled:opacity-50"
            aria-label="Close dialog"
          >
            <XMarkIcon className="h-5 w-5" />
          </button>
        </div>

        {/* Content */}
        <div className="mb-6">
          <p 
            id="dialog-description"
            className="text-secondary-700 dark:text-secondary-300 leading-relaxed"
          >
            {options.message}
          </p>
        </div>

        {/* Actions */}
        <div className="flex flex-col-reverse sm:flex-row sm:justify-end space-y-2 space-y-reverse sm:space-y-0 sm:space-x-3">
          <button
            onClick={handleCancel}
            disabled={isLoading}
            className="w-full sm:w-auto px-4 py-2 text-sm font-medium text-secondary-700 dark:text-secondary-300 hover:bg-secondary-100 dark:hover:bg-secondary-700 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {options.cancelText}
          </button>
          
          <LoadingButton
            onClick={handleConfirm}
            loading={isLoading}
            loadingText="Processing..."
            variant={dialogStyles.buttonClass}
            className="w-full sm:w-auto"
            disabled={isLoading}
          >
            {options.confirmText}
          </LoadingButton>
        </div>
      </div>
    </div>
  );
};

// Convenience hooks for common dialog types
export const useDeleteConfirmation = () => {
  const { confirm } = useConfirmDialog();

  return useCallback((
    itemName: string,
    onConfirm: () => void | Promise<void>,
    options?: Partial<ConfirmDialogOptions>
  ) => {
    confirm({
      type: 'danger',
      title: 'Delete Confirmation',
      message: `Are you sure you want to delete "${itemName}"? This action cannot be undone.`,
      confirmText: 'Delete',
      cancelText: 'Cancel',
      destructive: true,
      onConfirm,
      ...options,
    });
  }, [confirm]);
};

export const useRemoveConfirmation = () => {
  const { confirm } = useConfirmDialog();

  return useCallback((
    itemName: string,
    onConfirm: () => void | Promise<void>,
    options?: Partial<ConfirmDialogOptions>
  ) => {
    confirm({
      type: 'warning',
      title: 'Remove Confirmation',
      message: `Are you sure you want to remove "${itemName}"? You can add it back later if needed.`,
      confirmText: 'Remove',
      cancelText: 'Cancel',
      onConfirm,
      ...options,
    });
  }, [confirm]);
};

export const useUnsavedChangesConfirmation = () => {
  const { confirm } = useConfirmDialog();

  return useCallback((
    onConfirm: () => void | Promise<void>,
    options?: Partial<ConfirmDialogOptions>
  ) => {
    confirm({
      type: 'warning',
      title: 'Unsaved Changes',
      message: 'You have unsaved changes. Are you sure you want to leave without saving?',
      confirmText: 'Leave anyway',
      cancelText: 'Stay',
      onConfirm,
      ...options,
    });
  }, [confirm]);
};