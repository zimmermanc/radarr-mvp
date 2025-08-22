import { useAuth } from '../contexts/AuthContext';
import type { AuthState } from '../types/auth';

/**
 * Convenience hook for accessing authentication state
 */
export function useAuthState(): AuthState & {
  isLoading: boolean;
  login: ReturnType<typeof useAuth>['login'];
  logout: ReturnType<typeof useAuth>['logout'];
} {
  const { authState, isLoading, login, logout } = useAuth();
  
  return {
    ...authState,
    isLoading,
    login,
    logout
  };
}

/**
 * Hook for checking if user is authenticated
 */
export function useIsAuthenticated(): boolean {
  const { authState } = useAuth();
  return authState.isAuthenticated;
}

/**
 * Hook for getting current user info
 */
export function useCurrentUser() {
  const { authState } = useAuth();
  return authState.user;
}

/**
 * Hook for getting current API key
 */
export function useApiKey(): string | null {
  const { authState } = useAuth();
  return authState.apiKey;
}