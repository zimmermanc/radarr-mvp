import React, { createContext, useContext, useState, useCallback, useEffect } from 'react';

// Global UI state interface
interface UIState {
  // Loading states
  isLoading: boolean;
  loadingMessage?: string;
  
  // Sidebar
  sidebarOpen: boolean;
  sidebarCollapsed: boolean;
  
  // Mobile responsiveness
  isMobile: boolean;
  
  // Page state
  pageTitle: string;
  breadcrumbs: Breadcrumb[];
  
  // Global modals/overlays
  modalStack: string[];
  
  // Notifications
  notificationCount: number;
  
  // Search
  globalSearchOpen: boolean;
  
  // Settings
  compactMode: boolean;
  animations: boolean;
}

interface Breadcrumb {
  label: string;
  href?: string;
  icon?: React.ReactNode;
}

interface UIContextType extends UIState {
  // Loading actions
  setLoading: (loading: boolean, message?: string) => void;
  
  // Sidebar actions
  setSidebarOpen: (open: boolean) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  toggleSidebarCollapsed: () => void;
  
  // Page actions
  setPageTitle: (title: string) => void;
  setBreadcrumbs: (breadcrumbs: Breadcrumb[]) => void;
  addBreadcrumb: (breadcrumb: Breadcrumb) => void;
  
  // Modal actions
  pushModal: (modalId: string) => void;
  popModal: () => void;
  clearModals: () => void;
  isModalOpen: (modalId: string) => boolean;
  
  // Notification actions
  setNotificationCount: (count: number) => void;
  incrementNotifications: () => void;
  clearNotifications: () => void;
  
  // Search actions
  setGlobalSearchOpen: (open: boolean) => void;
  toggleGlobalSearch: () => void;
  
  // Settings actions
  setCompactMode: (compact: boolean) => void;
  setAnimations: (enabled: boolean) => void;
  
  // Utility actions
  resetUI: () => void;
}

const defaultState: UIState = {
  isLoading: false,
  loadingMessage: undefined,
  sidebarOpen: true,
  sidebarCollapsed: false,
  isMobile: false,
  pageTitle: 'Radarr',
  breadcrumbs: [],
  modalStack: [],
  notificationCount: 0,
  globalSearchOpen: false,
  compactMode: false,
  animations: true,
};

const UIContext = createContext<UIContextType | undefined>(undefined);

export const useUI = () => {
  const context = useContext(UIContext);
  if (!context) {
    throw new Error('useUI must be used within a UIProvider');
  }
  return context;
};

interface UIProviderProps {
  children: React.ReactNode;
  storageKey?: string;
}

export const UIProvider: React.FC<UIProviderProps> = ({ 
  children, 
  storageKey = 'radarr-ui-state' 
}) => {
  const [uiState, setUIState] = useState<UIState>(() => {
    if (typeof window === 'undefined') return defaultState;
    
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) {
        const parsed = JSON.parse(stored);
        // Only restore certain persisted settings
        return {
          ...defaultState,
          sidebarCollapsed: parsed.sidebarCollapsed ?? defaultState.sidebarCollapsed,
          compactMode: parsed.compactMode ?? defaultState.compactMode,
          animations: parsed.animations ?? defaultState.animations,
        };
      }
    } catch (error) {
      console.warn('Failed to load UI state from localStorage:', error);
    }
    
    return defaultState;
  });

  // Persist certain UI settings
  const persistSettings = useCallback(() => {
    try {
      const settingsToPersist = {
        sidebarCollapsed: uiState.sidebarCollapsed,
        compactMode: uiState.compactMode,
        animations: uiState.animations,
      };
      localStorage.setItem(storageKey, JSON.stringify(settingsToPersist));
    } catch (error) {
      console.warn('Failed to save UI state to localStorage:', error);
    }
  }, [uiState.sidebarCollapsed, uiState.compactMode, uiState.animations, storageKey]);

  // Detect mobile devices
  useEffect(() => {
    const checkMobile = () => {
      const isMobile = window.innerWidth < 768; // md breakpoint
      setUIState(prev => {
        const newState = { ...prev, isMobile };
        // Auto-collapse sidebar on mobile
        if (isMobile && prev.sidebarOpen) {
          newState.sidebarOpen = false;
        }
        return newState;
      });
    };

    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  // Persist settings when they change
  useEffect(() => {
    persistSettings();
  }, [persistSettings]);

  // Actions
  const setLoading = useCallback((loading: boolean, message?: string) => {
    setUIState(prev => ({ ...prev, isLoading: loading, loadingMessage: message }));
  }, []);

  const setSidebarOpen = useCallback((open: boolean) => {
    setUIState(prev => ({ ...prev, sidebarOpen: open }));
  }, []);

  const toggleSidebar = useCallback(() => {
    setUIState(prev => ({ ...prev, sidebarOpen: !prev.sidebarOpen }));
  }, []);

  const setSidebarCollapsed = useCallback((collapsed: boolean) => {
    setUIState(prev => ({ ...prev, sidebarCollapsed: collapsed }));
  }, []);

  const toggleSidebarCollapsed = useCallback(() => {
    setUIState(prev => ({ ...prev, sidebarCollapsed: !prev.sidebarCollapsed }));
  }, []);

  const setPageTitle = useCallback((title: string) => {
    setUIState(prev => ({ ...prev, pageTitle: title }));
    
    // Update document title
    document.title = title === 'Radarr' ? 'Radarr' : `${title} - Radarr`;
  }, []);

  const setBreadcrumbs = useCallback((breadcrumbs: Breadcrumb[]) => {
    setUIState(prev => ({ ...prev, breadcrumbs }));
  }, []);

  const addBreadcrumb = useCallback((breadcrumb: Breadcrumb) => {
    setUIState(prev => ({ 
      ...prev, 
      breadcrumbs: [...prev.breadcrumbs, breadcrumb] 
    }));
  }, []);

  const pushModal = useCallback((modalId: string) => {
    setUIState(prev => ({ 
      ...prev, 
      modalStack: [...prev.modalStack, modalId] 
    }));
  }, []);

  const popModal = useCallback(() => {
    setUIState(prev => ({ 
      ...prev, 
      modalStack: prev.modalStack.slice(0, -1) 
    }));
  }, []);

  const clearModals = useCallback(() => {
    setUIState(prev => ({ ...prev, modalStack: [] }));
  }, []);

  const isModalOpen = useCallback((modalId: string) => {
    return uiState.modalStack.includes(modalId);
  }, [uiState.modalStack]);

  const setNotificationCount = useCallback((count: number) => {
    setUIState(prev => ({ ...prev, notificationCount: Math.max(0, count) }));
  }, []);

  const incrementNotifications = useCallback(() => {
    setUIState(prev => ({ ...prev, notificationCount: prev.notificationCount + 1 }));
  }, []);

  const clearNotifications = useCallback(() => {
    setUIState(prev => ({ ...prev, notificationCount: 0 }));
  }, []);

  const setGlobalSearchOpen = useCallback((open: boolean) => {
    setUIState(prev => ({ ...prev, globalSearchOpen: open }));
  }, []);

  const toggleGlobalSearch = useCallback(() => {
    setUIState(prev => ({ ...prev, globalSearchOpen: !prev.globalSearchOpen }));
  }, []);

  const setCompactMode = useCallback((compact: boolean) => {
    setUIState(prev => ({ ...prev, compactMode: compact }));
  }, []);

  const setAnimations = useCallback((enabled: boolean) => {
    setUIState(prev => ({ ...prev, animations: enabled }));
    
    // Add/remove animation class to document
    if (enabled) {
      document.documentElement.classList.remove('no-animations');
    } else {
      document.documentElement.classList.add('no-animations');
    }
  }, []);

  const resetUI = useCallback(() => {
    setUIState({
      ...defaultState,
      isMobile: uiState.isMobile, // Keep current mobile state
    });
  }, [uiState.isMobile]);

  const value: UIContextType = {
    ...uiState,
    setLoading,
    setSidebarOpen,
    toggleSidebar,
    setSidebarCollapsed,
    toggleSidebarCollapsed,
    setPageTitle,
    setBreadcrumbs,
    addBreadcrumb,
    pushModal,
    popModal,
    clearModals,
    isModalOpen,
    setNotificationCount,
    incrementNotifications,
    clearNotifications,
    setGlobalSearchOpen,
    toggleGlobalSearch,
    setCompactMode,
    setAnimations,
    resetUI,
  };

  return (
    <UIContext.Provider value={value}>
      {children}
    </UIContext.Provider>
  );
};

// Convenience hooks
export const usePageTitle = (title: string) => {
  const { setPageTitle } = useUI();
  
  useEffect(() => {
    setPageTitle(title);
    return () => setPageTitle('Radarr');
  }, [title, setPageTitle]);
};

export const useBreadcrumbs = (breadcrumbs: Breadcrumb[]) => {
  const { setBreadcrumbs } = useUI();
  
  useEffect(() => {
    setBreadcrumbs(breadcrumbs);
    return () => setBreadcrumbs([]);
  }, [breadcrumbs, setBreadcrumbs]);
};

export const useLoading = () => {
  const { isLoading, setLoading } = useUI();
  return { isLoading, setLoading };
};