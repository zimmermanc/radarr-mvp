// Authentication Types
export interface AuthState {
  isAuthenticated: boolean;
  apiKey: string | null;
  user: User | null;
}

export interface User {
  username: string;
  lastLogin: string;
}

export interface LoginCredentials {
  username?: string;
  password?: string;
  apiKey?: string;
}

export interface LoginResponse {
  success: boolean;
  message?: string;
  user?: User;
}

export interface AuthContextType {
  authState: AuthState;
  login: (credentials: LoginCredentials) => Promise<LoginResponse>;
  logout: () => void;
  isLoading: boolean;
}

// Local storage keys
export const AUTH_STORAGE_KEYS = {
  API_KEY: 'radarr_api_key',
  USER: 'radarr_user',
  LAST_LOGIN: 'radarr_last_login'
} as const;