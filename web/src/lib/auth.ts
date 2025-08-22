import { AUTH_STORAGE_KEYS, type User, type LoginCredentials } from '../types/auth';
import { createApiClient } from './api';

/**
 * Authentication service for managing user sessions and API keys
 */
export class AuthService {
  private static readonly PRODUCTION_API_KEY = 'secure_production_api_key_2025';
  private static readonly DEFAULT_USERNAME = 'admin';
  private static readonly DEFAULT_PASSWORD = 'admin';

  /**
   * Validates credentials against the backend API
   */
  static async validateCredentials(credentials: LoginCredentials): Promise<boolean> {
    try {
      // If API key is provided, validate it directly
      if (credentials.apiKey) {
        return await this.validateApiKey(credentials.apiKey);
      }

      // For username/password, check against defaults and validate with production API key
      if (credentials.username && credentials.password) {
        const isValidUser = credentials.username === this.DEFAULT_USERNAME && 
                           credentials.password === this.DEFAULT_PASSWORD;
        
        if (isValidUser) {
          return await this.validateApiKey(this.PRODUCTION_API_KEY);
        }
      }

      return false;
    } catch (error) {
      console.error('Authentication validation failed:', error);
      return false;
    }
  }

  /**
   * Validates API key by testing against the backend
   */
  private static async validateApiKey(apiKey: string): Promise<boolean> {
    try {
      const baseUrl = import.meta.env.VITE_API_BASE_URL || 
                      (import.meta.env.DEV ? '' : `${window.location.protocol}//${window.location.hostname}:7878`);
      
      const testClient = createApiClient({
        baseUrl,
        apiKey,
        timeout: 10000
      });

      const result = await testClient.testConnection();
      return result;
    } catch (error) {
      console.error('API key validation failed:', error);
      return false;
    }
  }

  /**
   * Stores authentication data in localStorage
   */
  static storeAuth(apiKey: string, user: User): void {
    try {
      localStorage.setItem(AUTH_STORAGE_KEYS.API_KEY, apiKey);
      localStorage.setItem(AUTH_STORAGE_KEYS.USER, JSON.stringify(user));
      localStorage.setItem(AUTH_STORAGE_KEYS.LAST_LOGIN, new Date().toISOString());
    } catch (error) {
      console.error('Failed to store authentication data:', error);
    }
  }

  /**
   * Retrieves stored authentication data
   */
  static getStoredAuth(): { apiKey: string | null; user: User | null } {
    try {
      const apiKey = localStorage.getItem(AUTH_STORAGE_KEYS.API_KEY);
      const userJson = localStorage.getItem(AUTH_STORAGE_KEYS.USER);
      const user = userJson ? JSON.parse(userJson) : null;

      return { apiKey, user };
    } catch (error) {
      console.error('Failed to retrieve authentication data:', error);
      return { apiKey: null, user: null };
    }
  }

  /**
   * Clears all authentication data
   */
  static clearAuth(): void {
    try {
      localStorage.removeItem(AUTH_STORAGE_KEYS.API_KEY);
      localStorage.removeItem(AUTH_STORAGE_KEYS.USER);
      localStorage.removeItem(AUTH_STORAGE_KEYS.LAST_LOGIN);
    } catch (error) {
      console.error('Failed to clear authentication data:', error);
    }
  }

  /**
   * Creates a user object from credentials
   */
  static createUser(credentials: LoginCredentials): User {
    return {
      username: credentials.username || 'API User',
      lastLogin: new Date().toISOString()
    };
  }

  /**
   * Gets the API key to use for authentication
   */
  static getApiKey(credentials: LoginCredentials): string {
    return credentials.apiKey || this.PRODUCTION_API_KEY;
  }

  /**
   * Checks if current stored authentication is still valid
   */
  static async validateStoredAuth(): Promise<boolean> {
    const { apiKey } = this.getStoredAuth();
    if (!apiKey) return false;

    return await this.validateApiKey(apiKey);
  }
}