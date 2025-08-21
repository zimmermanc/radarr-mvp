# Radarr MVP Frontend

A modern React + TypeScript frontend for the Radarr MVP, built with Vite and styled with Tailwind CSS.

## Features

- **Modern React 18** with TypeScript for type safety
- **Vite** for fast development and build experience
- **Tailwind CSS** for responsive, utility-first styling
- **React Router** for client-side navigation
- **Axios-based API client** with authentication and error handling
- **Heroicons** for beautiful, consistent icons
- **Dark mode ready** with Tailwind's dark mode utilities

## Project Structure

```
web/
├── src/
│   ├── components/
│   │   └── layout/          # Layout components (Header, Sidebar, Layout)
│   ├── pages/               # Page components (Dashboard, Movies, etc.)
│   ├── lib/                 # Utility libraries (API client)
│   ├── types/               # TypeScript type definitions
│   └── index.css           # Global styles with Tailwind
├── public/                  # Static assets
└── package.json
```

## Pages Implemented

- **Dashboard** - Overview with stats and recent movies
- **Movies** - Movie library with search and filtering
- **Add Movie** - Search and add new movies to collection
- **Settings** - API configuration and connection testing

## API Integration

The frontend integrates with the Radarr backend API with the following features:

- **Authentication**: X-Api-Key header authentication
- **Error Handling**: Comprehensive error handling with user-friendly messages
- **Type Safety**: Full TypeScript types for all API responses
- **Connection Testing**: Built-in API connection testing in settings

### Endpoints Used

- `GET /health` - Health check
- `GET /api/v3/movie` - List movies with filtering and sorting
- `GET /api/v3/movie/{id}` - Get specific movie details
- `POST /api/v3/movie` - Add new movie to collection
- `GET /api/v3/movie/lookup` - Search for movies to add
- `GET /api/v3/qualityprofile` - Get quality profiles

## Configuration

Environment variables are configured in `.env`:

```env
# Radarr API Configuration
VITE_API_BASE_URL=http://localhost:7878
VITE_API_KEY=mysecurekey123

# Application Configuration
VITE_APP_TITLE=Radarr MVP
VITE_APP_VERSION=1.0.0

# Development Configuration
VITE_DEBUG=true
VITE_LOG_LEVEL=debug
```

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## API Client Usage

The API client is configured as a singleton and can be used throughout the application:

```typescript
import { radarrApi, isApiError } from '../lib/api';

// Get movies
const response = await radarrApi.getMovies();
if (isApiError(response)) {
  console.error('Error:', response.error.message);
} else {
  console.log('Movies:', response.data.movies);
}

// Test connection
const isConnected = await radarrApi.testConnection();
```

## Styling

The application uses Tailwind CSS with a custom design system:

- **Primary Colors**: Blue color palette for actions and highlights
- **Secondary Colors**: Gray color palette for text and backgrounds
- **Status Colors**: Success (green), warning (yellow), error (red)
- **Custom Components**: Pre-built component classes for buttons, cards, forms
- **Dark Mode**: Built-in dark mode support with `dark:` variants

## Responsive Design

The application is fully responsive with:

- **Mobile-first approach** with responsive breakpoints
- **Collapsible sidebar** on mobile devices
- **Flexible grid layouts** that adapt to screen size
- **Touch-friendly interactions** for mobile devices

## Performance Features

- **Code splitting** with React Router and Vite
- **Lazy loading** of images and components
- **Optimized bundling** with Vite's build system
- **Modern JavaScript** with ES modules and tree shaking

## Browser Support

- Modern browsers with ES2020+ support
- Chrome 88+, Firefox 84+, Safari 14+, Edge 88+

## Development Status

✅ **Completed:**
- Project setup and configuration
- Core routing and navigation
- API client with authentication
- Dashboard with movie statistics
- Movie library with search/filtering
- Add movie functionality
- Settings page with API configuration
- Responsive layout and styling

🚧 **Future Enhancements:**
- Activity/Queue pages
- Movie detail modal/page
- Bulk operations
- Real-time updates
- Advanced filtering options
- Download management integration
