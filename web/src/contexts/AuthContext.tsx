import React, { createContext, useContext, useReducer, useEffect } from 'react';
import type { ReactNode } from 'react';
import type { AuthState, AuthContextType, LoginCredentials, LoginResponse, User } from '../types/auth';
import { AuthService } from '../lib/auth';
import { radarrApi } from '../lib/api';

// Initial state
const initialState: AuthState = {
  isAuthenticated: false,
  apiKey: null,
  user: null
};

// Auth actions
type AuthAction =
  | { type: 'LOGIN_START' }
  | { type: 'LOGIN_SUCCESS'; payload: { apiKey: string; user: User } }
  | { type: 'LOGIN_FAILURE' }
  | { type: 'LOGOUT' }
  | { type: 'RESTORE_SESSION'; payload: { apiKey: string; user: User } };

// Auth reducer
function authReducer(state: AuthState, action: AuthAction): AuthState {
  switch (action.type) {
    case 'LOGIN_START':
      return state;
    case 'LOGIN_SUCCESS':
      return {
        isAuthenticated: true,
        apiKey: action.payload.apiKey,
        user: action.payload.user
      };
    case 'LOGIN_FAILURE':
      return {
        isAuthenticated: false,
        apiKey: null,
        user: null
      };
    case 'LOGOUT':
      return {
        isAuthenticated: false,
        apiKey: null,
        user: null
      };
    case 'RESTORE_SESSION':
      return {
        isAuthenticated: true,
        apiKey: action.payload.apiKey,
        user: action.payload.user
      };
    default:
      return state;
  }
}

// Create context
const AuthContext = createContext<AuthContextType | undefined>(undefined);

// Auth provider component
interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [authState, dispatch] = useReducer(authReducer, initialState);
  const [isLoading, setIsLoading] = React.useState(true);

  // Restore session on app start
  useEffect(() => {
    const restoreSession = async () => {
      try {
        const { apiKey, user } = AuthService.getStoredAuth();
        
        if (apiKey && user) {
          // Validate stored credentials
          const isValid = await AuthService.validateStoredAuth();
          
          if (isValid) {
            // Update API client with stored key
            radarrApi.updateConfig({ apiKey });
            
            dispatch({
              type: 'RESTORE_SESSION',
              payload: { apiKey, user }
            });
          } else {
            // Clear invalid stored data
            AuthService.clearAuth();
          }
        }
      } catch (error) {
        console.error('Failed to restore session:', error);
        AuthService.clearAuth();
      } finally {
        setIsLoading(false);
      }
    };

    restoreSession();
  }, []);

  // Login function
  const login = async (credentials: LoginCredentials): Promise<LoginResponse> => {
    try {
      setIsLoading(true);
      dispatch({ type: 'LOGIN_START' });

      // Validate credentials
      const isValid = await AuthService.validateCredentials(credentials);
      
      if (!isValid) {
        dispatch({ type: 'LOGIN_FAILURE' });
        return {
          success: false,
          message: 'Invalid credentials. Please check your username/password or API key.'
        };
      }

      // Create user and get API key
      const user = AuthService.createUser(credentials);
      const apiKey = AuthService.getApiKey(credentials);

      // Store authentication data
      AuthService.storeAuth(apiKey, user);

      // Update API client
      radarrApi.updateConfig({ apiKey });

      // Update state
      dispatch({
        type: 'LOGIN_SUCCESS',
        payload: { apiKey, user }
      });

      return {
        success: true,
        user
      };
    } catch (error) {
      console.error('Login failed:', error);
      dispatch({ type: 'LOGIN_FAILURE' });
      
      return {
        success: false,
        message: 'Login failed. Please check your connection and try again.'
      };
    } finally {
      setIsLoading(false);
    }
  };

  // Logout function
  const logout = () => {
    AuthService.clearAuth();
    
    // Reset API client to use default key
    radarrApi.updateConfig({ 
      apiKey: import.meta.env.VITE_API_KEY || 'mysecurekey123' 
    });
    
    dispatch({ type: 'LOGOUT' });
  };

  const contextValue: AuthContextType = {
    authState,
    login,
    logout,
    isLoading
  };

  return (
    <AuthContext.Provider value={contextValue}>
      {children}
    </AuthContext.Provider>
  );
}

// Hook to use auth context
export function useAuth(): AuthContextType {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}