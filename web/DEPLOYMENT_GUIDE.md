# Radarr Web UI - Deployment Guide

## Problem Solved

The Radarr web UI now properly handles API connections regardless of deployment IP addresses. Previously, hardcoded IP addresses in the configuration would break when users accessed the UI from different interfaces (localhost, LAN IPs, etc.), causing CORS errors and failed API connections.

## Solution Architecture

### Development Mode
- **Web UI**: Runs on `0.0.0.0:5173` (accessible from any interface)
- **Backend API**: Runs on `localhost:7878` (secure, not exposed externally)
- **Proxy**: Vite dev server proxies `/api` and `/health` requests to `localhost:7878`
- **CORS**: Eliminated by using proxy - all requests appear to come from the same origin

### Production Mode  
- **Web UI**: Served by web server or CDN
- **Backend API**: Runs on configured host/port
- **URLs**: Relative URLs automatically resolve to the correct backend location
- **CORS**: Properly configured for production domains

## Configuration

### Environment Variables (.env)

```bash
# Development (recommended)
VITE_API_BASE_URL=                    # Empty = use proxy
VITE_API_KEY=mysecurekey123

# Production
VITE_API_BASE_URL=https://api.radarr.com    # Explicit API URL
VITE_API_KEY=your-secure-api-key
```

### Vite Configuration (vite.config.ts)

The Vite dev server is configured to:
- Listen on `0.0.0.0:5173` (accessible from any network interface)
- Proxy `/api/*` and `/health` requests to `localhost:7878`
- Log proxy requests for debugging
- Handle proxy errors gracefully

### API Client Logic (src/lib/api.ts)

Smart base URL detection:
```typescript
const getBaseUrl = (): string => {
  // Production: Use explicit URL if configured
  if (import.meta.env.VITE_API_BASE_URL) {
    return import.meta.env.VITE_API_BASE_URL;
  }
  
  // Development: Use relative URLs (proxied)
  if (import.meta.env.DEV) {
    return ''; // Relative URLs
  }
  
  // Fallback: Auto-detect from current location
  return `${window.location.protocol}//${window.location.hostname}:7878`;
};
```

### Backend CORS Configuration (crates/api/src/security.rs)

Enhanced CORS support for development:
- Automatically allows common development origins
- Supports `localhost:5173`, `127.0.0.1:5173`, `0.0.0.0:5173`
- Production mode strictly validates origins
- Environment-based configuration

## Deployment Instructions

### Development Setup

1. **Start Backend API**:
   ```bash
   cd unified-radarr
   RADARR_API_KEY=mysecurekey123 cargo run --bin radarr-mvp
   ```

2. **Start Web UI**:
   ```bash
   cd web
   npm run dev
   ```

3. **Access from any interface**:
   - `http://localhost:5173` ✅
   - `http://127.0.0.1:5173` ✅
   - `http://192.168.1.100:5173` ✅ (your LAN IP)
   - All will work without CORS issues

### Production Deployment

1. **Build Web UI**:
   ```bash
   cd web
   npm run build
   ```

2. **Configure Environment**:
   ```bash
   export VITE_API_BASE_URL=https://api.radarr.com
   export VITE_API_KEY=your-production-key
   npm run build
   ```

3. **Deploy**:
   - Serve `dist/` folder with any web server
   - Configure reverse proxy if needed
   - Set appropriate CORS origins on backend

## Testing the Solution

A `ConnectionTest` component has been added to verify the API connection:
- Tests health endpoint
- Tests movies endpoint  
- Shows connection status and debugging info
- Displays current configuration

Access it at: Dashboard page → API Connection Test section

## Key Benefits

1. **No Hardcoded IPs**: Works with any IP address configuration
2. **CORS-Free Development**: Proxy eliminates cross-origin issues
3. **Secure by Default**: Backend API not exposed externally in development
4. **Production Ready**: Proper CORS and URL handling for production
5. **User Portable**: Same configuration works for all developers/users
6. **Debugging Friendly**: Built-in connection testing and logging

## Troubleshooting

### Common Issues

**Problem**: `ECONNREFUSED localhost:7878`
**Solution**: Ensure backend API is running with `RADARR_API_KEY=mysecurekey123 cargo run --bin radarr-mvp`

**Problem**: CORS errors in development
**Solution**: Ensure `VITE_API_BASE_URL` is empty in `.env` to use proxy

**Problem**: 401 Unauthorized
**Solution**: Verify `VITE_API_KEY` matches `RADARR_API_KEY` on backend

**Problem**: Can't access from LAN IP
**Solution**: Vite dev server runs on `0.0.0.0:5173` - check firewall settings

### Debug Information

The ConnectionTest component shows:
- Current base URL configuration
- Environment mode (development/production)
- Current browser location
- API connection status
- Available endpoints

## Files Modified

- `/web/.env` - Removed hardcoded IP
- `/web/vite.config.ts` - Added proxy configuration
- `/web/src/lib/api.ts` - Smart base URL detection
- `/crates/api/src/security.rs` - Enhanced CORS support
- `/web/.env.example` - Updated with documentation

This solution provides a robust, portable API connection system that works reliably across different deployment scenarios and user environments.