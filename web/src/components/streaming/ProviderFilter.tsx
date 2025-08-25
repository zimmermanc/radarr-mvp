import React, { useState, useEffect } from 'react';
import { Filter, X } from 'lucide-react';
import { getStreamingApi } from '../../lib/streamingApi';
import type { StreamingProvider } from '../../types/streaming';

interface ProviderFilterProps {
  region?: string;
  selectedProviders: string[];
  onProvidersChange: (providers: string[]) => void;
  className?: string;
}

export const ProviderFilter: React.FC<ProviderFilterProps> = ({
  region = 'US',
  selectedProviders,
  onProvidersChange,
  className = '',
}) => {
  const [providers, setProviders] = useState<StreamingProvider[]>([]);
  const [loading, setLoading] = useState(true);
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    fetchProviders();
  }, [region]);

  const fetchProviders = async () => {
    try {
      setLoading(true);
      const api = getStreamingApi();
      const response = await api.getProviders(region);
      // Defensive programming: ensure providers is always an array
      const providers = response?.providers || response?.data?.providers || [];
      setProviders(Array.isArray(providers) ? providers : []);
    } catch (err) {
      console.error('Failed to fetch providers:', err);
      setProviders([]); // Set empty array on error
    } finally {
      setLoading(false);
    }
  };

  const toggleProvider = (providerName: string) => {
    // Defensive programming: ensure selectedProviders is always an array
    const safeSelected = Array.isArray(selectedProviders) ? selectedProviders : [];
    if (safeSelected.includes(providerName)) {
      onProvidersChange(safeSelected.filter((p) => p !== providerName));
    } else {
      onProvidersChange([...safeSelected, providerName]);
    }
  };

  const clearFilters = () => {
    onProvidersChange([]);
  };

  // Popular providers to show first
  const popularProviders = ['Netflix', 'Disney Plus', 'Amazon Prime Video', 'Hulu', 'Apple TV Plus', 'HBO Max'];
  // Defensive programming: ensure providers is always an array before spreading
  const safeProviders = Array.isArray(providers) ? providers : [];
  const sortedProviders = [...safeProviders].sort((a, b) => {
    const aIsPopular = popularProviders.includes(a.name);
    const bIsPopular = popularProviders.includes(b.name);
    if (aIsPopular && !bIsPopular) return -1;
    if (!aIsPopular && bIsPopular) return 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <div className={`relative ${className}`}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2 px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
      >
        <Filter className="w-4 h-4" />
        <span>Filter by Provider</span>
        {selectedProviders.length > 0 && (
          <span className="ml-1 px-2 py-0.5 bg-blue-500 text-white text-xs rounded-full">
            {selectedProviders.length}
          </span>
        )}
      </button>

      {isOpen && (
        <>
          <div
            className="fixed inset-0 z-40"
            onClick={() => setIsOpen(false)}
          />
          
          <div className="absolute top-full mt-2 w-80 max-h-96 overflow-auto bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl z-50">
            <div className="sticky top-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 p-3">
              <div className="flex items-center justify-between">
                <h3 className="font-semibold text-gray-900 dark:text-white">
                  Streaming Providers
                </h3>
                {selectedProviders.length > 0 && (
                  <button
                    onClick={clearFilters}
                    className="text-sm text-blue-600 dark:text-blue-400 hover:underline"
                  >
                    Clear all
                  </button>
                )}
              </div>
            </div>

            {loading ? (
              <div className="p-4 text-center">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto"></div>
              </div>
            ) : (
              <div className="p-2">
                {sortedProviders.map((provider) => {
                  const isSelected = selectedProviders.includes(provider.name);
                  const isPopular = popularProviders.includes(provider.name);
                  
                  return (
                    <label
                      key={provider.name}
                      className="flex items-center gap-3 px-2 py-2 hover:bg-gray-50 dark:hover:bg-gray-700/50 rounded cursor-pointer"
                    >
                      <input
                        type="checkbox"
                        checked={isSelected}
                        onChange={() => toggleProvider(provider.name)}
                        className="w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                      />
                      
                      <div className="flex items-center gap-2 flex-1">
                        {provider.logo_url ? (
                          <img
                            src={provider.logo_url}
                            alt={provider.name}
                            className="w-6 h-6 object-contain"
                          />
                        ) : (
                          <div className="w-6 h-6 bg-gray-200 dark:bg-gray-700 rounded"></div>
                        )}
                        
                        <span className="text-sm text-gray-900 dark:text-white">
                          {provider.name}
                        </span>
                        
                        {isPopular && (
                          <span className="text-xs px-2 py-0.5 bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400 rounded">
                            Popular
                          </span>
                        )}
                      </div>
                    </label>
                  );
                })}
              </div>
            )}
          </div>
        </>
      )}

      {Array.isArray(selectedProviders) && selectedProviders.length > 0 && (
        <div className="flex flex-wrap gap-2 mt-2">
          {selectedProviders.map((provider) => (
            <span
              key={provider}
              className="inline-flex items-center gap-1 px-3 py-1 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 rounded-full text-sm"
            >
              {provider}
              <button
                onClick={() => toggleProvider(provider)}
                className="hover:bg-blue-200 dark:hover:bg-blue-800/50 rounded-full p-0.5"
              >
                <X className="w-3 h-3" />
              </button>
            </span>
          ))}
        </div>
      )}
    </div>
  );
};