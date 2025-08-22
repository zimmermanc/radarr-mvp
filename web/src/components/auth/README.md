# Authentication System

This directory contains the authentication components and logic for the Radarr MVP web interface.

## Components

### LoginPage
- Main login interface with support for both username/password and API key authentication
- Responsive design matching the application's dark theme
- Form validation and error handling
- Automatic redirection after successful login

### ProtectedRoute
- Higher-order component that wraps protected routes
- Automatically redirects unauthenticated users to the login page
- Preserves intended destination for redirect after login
- Shows loading spinner during authentication check

### LogoutButton
- Reusable logout button component
- Available in button and link variants
- Shows success notification on logout
- Can be used anywhere in the application

## Authentication Flow

1. **Initial Load**: AuthContext checks localStorage for stored credentials
2. **Validation**: Stored credentials are validated against the backend API
3. **Login**: User can authenticate with username/password or API key
4. **Token Storage**: API key is securely stored in localStorage
5. **API Integration**: API client is automatically updated with the authenticated key
6. **Logout**: Clears all stored authentication data and resets API client

## Default Credentials

- **Username**: `admin`
- **Password**: `admin`
- **API Key**: `secure_production_api_key_2025`

## Security Features

- API key validation against backend before storing
- Automatic token expiration handling
- Secure localStorage management
- CSRF protection through API key header
- Input validation and sanitization

## Usage

```tsx
import { useAuth } from '../../contexts/AuthContext';
import { ProtectedRoute } from './components/auth';

// In your app routing
<Route path="/dashboard" element={
  <ProtectedRoute>
    <Dashboard />
  </ProtectedRoute>
} />

// In components
const { authState, login, logout } = useAuth();
```