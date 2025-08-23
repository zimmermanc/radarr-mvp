import React, { useState, useEffect } from 'react';
import { Play, DollarSign, Tv, ShoppingBag, AlertCircle } from 'lucide-react';
import { getStreamingApi } from '../../lib/streamingApi';
import type { AvailabilityItem, ServiceType } from '../../types/streaming';

interface StreamingAvailabilityProps {
  tmdbId: number;
  region?: string;
  compact?: boolean;
}

export const StreamingAvailability: React.FC<StreamingAvailabilityProps> = ({
  tmdbId,
  region = 'US',
  compact = false,
}) => {
  const [items, setItems] = useState<AvailabilityItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (tmdbId) {
      fetchAvailability();
    }
  }, [tmdbId, region]);

  const fetchAvailability = async () => {
    try {
      setLoading(true);
      setError(null);
      const api = getStreamingApi();
      const response = await api.getAvailability(tmdbId, region);
      setItems(response.items);
    } catch (err) {
      console.error('Failed to fetch availability:', err);
      setError('Failed to load streaming availability');
    } finally {
      setLoading(false);
    }
  };

  const getServiceTypeIcon = (type: ServiceType) => {
    switch (type) {
      case 'subscription':
        return <Play className="w-4 h-4" />;
      case 'rent':
        return <DollarSign className="w-4 h-4" />;
      case 'buy':
        return <ShoppingBag className="w-4 h-4" />;
      case 'free':
      case 'ads':
        return <Tv className="w-4 h-4" />;
      default:
        return <Play className="w-4 h-4" />;
    }
  };

  const getServiceTypeLabel = (type: ServiceType) => {
    switch (type) {
      case 'subscription':
        return 'Stream';
      case 'rent':
        return 'Rent';
      case 'buy':
        return 'Buy';
      case 'free':
        return 'Free';
      case 'ads':
        return 'Free with Ads';
      default:
        return type;
    }
  };

  const groupedItems = items.reduce((acc, item) => {
    const key = item.service_name;
    if (!acc[key]) {
      acc[key] = [];
    }
    acc[key].push(item);
    return acc;
  }, {} as Record<string, AvailabilityItem[]>);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-4">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center gap-2 text-red-600 dark:text-red-400 py-2">
        <AlertCircle className="w-4 h-4" />
        <span className="text-sm">{error}</span>
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="text-gray-500 dark:text-gray-400 py-2">
        <p className="text-sm">Not currently available for streaming in {region}</p>
      </div>
    );
  }

  if (compact) {
    // Compact view for movie cards
    const uniqueServices = Object.keys(groupedItems).slice(0, 4);
    return (
      <div className="flex flex-wrap gap-2">
        {uniqueServices.map((serviceName) => {
          const serviceItems = groupedItems[serviceName];
          const firstItem = serviceItems[0];
          return (
            <a
              key={serviceName}
              href={firstItem.deep_link}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded-md hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
              title={`Available on ${serviceName}`}
            >
              {firstItem.service_logo_url ? (
                <img
                  src={firstItem.service_logo_url}
                  alt={serviceName}
                  className="w-4 h-4 object-contain"
                />
              ) : (
                getServiceTypeIcon(firstItem.service_type)
              )}
              <span className="text-xs font-medium">{serviceName}</span>
            </a>
          );
        })}
        {Object.keys(groupedItems).length > 4 && (
          <span className="text-xs text-gray-500 dark:text-gray-400 py-1">
            +{Object.keys(groupedItems).length - 4} more
          </span>
        )}
      </div>
    );
  }

  // Full view for detail pages
  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
        Streaming Availability
      </h3>
      
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {Object.entries(groupedItems).map(([serviceName, serviceItems]) => (
          <div
            key={serviceName}
            className="border border-gray-200 dark:border-gray-700 rounded-lg p-3"
          >
            <div className="flex items-start justify-between mb-2">
              <div className="flex items-center gap-2">
                {serviceItems[0].service_logo_url ? (
                  <img
                    src={serviceItems[0].service_logo_url}
                    alt={serviceName}
                    className="w-8 h-8 object-contain"
                  />
                ) : (
                  <div className="w-8 h-8 bg-gray-200 dark:bg-gray-700 rounded flex items-center justify-center">
                    {getServiceTypeIcon(serviceItems[0].service_type)}
                  </div>
                )}
                <span className="font-medium text-gray-900 dark:text-white">
                  {serviceName}
                </span>
              </div>
            </div>

            <div className="space-y-1">
              {serviceItems.map((item, index) => (
                <a
                  key={`${item.service_name}-${item.service_type}-${index}`}
                  href={item.deep_link}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center justify-between p-2 rounded hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
                >
                  <div className="flex items-center gap-2">
                    {getServiceTypeIcon(item.service_type)}
                    <span className="text-sm text-gray-700 dark:text-gray-300">
                      {getServiceTypeLabel(item.service_type)}
                    </span>
                    {item.quality && (
                      <span className="text-xs px-2 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded">
                        {item.quality}
                      </span>
                    )}
                  </div>
                  
                  {item.price_amount && (
                    <span className="text-sm font-medium text-gray-900 dark:text-white">
                      {item.price_currency} {item.price_amount.toFixed(2)}
                    </span>
                  )}
                  
                  {item.leaving_date && (
                    <span className="text-xs text-orange-600 dark:text-orange-400">
                      Leaving {new Date(item.leaving_date).toLocaleDateString()}
                    </span>
                  )}
                </a>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};