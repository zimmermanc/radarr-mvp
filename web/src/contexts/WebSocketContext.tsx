import React, { createContext, useContext, useEffect, useState, useCallback, useRef } from 'react';
import { useToast } from '../components/ui/Toast';
import { useAuth } from './AuthContext';

interface WebSocketMessage {
  type: 'movie_added' | 'movie_updated' | 'movie_deleted' | 
        'download_started' | 'download_progress' | 'download_completed' | 'download_failed' |
        'import_started' | 'import_completed' | 'import_failed' |
        'health_check' | 'notification' | 'queue_update';
  payload: any;
  timestamp: string;
}

interface WebSocketContextType {
  isConnected: boolean;
  lastMessage: WebSocketMessage | null;
  sendMessage: (message: any) => void;
  subscribe: (type: string, handler: (payload: any) => void) => () => void;
  reconnect: () => void;
  connectionError: string | null;
}

const WebSocketContext = createContext<WebSocketContextType | undefined>(undefined);

export const useWebSocket = () => {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error('useWebSocket must be used within WebSocketProvider');
  }
  return context;
};

interface WebSocketProviderProps {
  children: React.ReactNode;
  url?: string;
}

export const WebSocketProvider: React.FC<WebSocketProviderProps> = ({ 
  children, 
  url = `ws://${window.location.hostname}:7878/ws` 
}) => {
  const [isConnected, setIsConnected] = useState(false);
  const [lastMessage, setLastMessage] = useState<WebSocketMessage | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);
  const subscribersRef = useRef<Map<string, Set<(payload: any) => void>>>(new Map());
  const { info, error: toastError } = useToast();
  const { authState } = useAuth();

  const connect = useCallback(() => {
    // Don't connect if not authenticated
    if (!authState.isAuthenticated || !authState.apiKey) {
      setConnectionError('Authentication required for WebSocket connection');
      return;
    }

    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    try {
      // Build WebSocket URL with API key
      const wsUrl = new URL(url);
      wsUrl.searchParams.set('apikey', authState.apiKey);
      
      setConnectionError(null);
      wsRef.current = new WebSocket(wsUrl.toString());

      wsRef.current.onopen = () => {
        setIsConnected(true);
        setConnectionError(null);
        info('Connected', 'Real-time updates connected');
        
        // Clear any pending reconnect
        if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current);
          reconnectTimeoutRef.current = null;
        }

        // Send initial subscription message
        sendMessage({ type: 'subscribe', channels: ['all'] });
      };

      wsRef.current.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as WebSocketMessage;
          setLastMessage(message);

          // Notify subscribers
          const handlers = subscribersRef.current.get(message.type);
          if (handlers) {
            handlers.forEach(handler => handler(message.payload));
          }

          // Also notify 'all' subscribers
          const allHandlers = subscribersRef.current.get('all');
          if (allHandlers) {
            allHandlers.forEach(handler => handler(message));
          }
        } catch (err) {
          console.error('Failed to parse WebSocket message:', err);
        }
      };

      wsRef.current.onerror = (err) => {
        console.error('WebSocket error:', err);
        setConnectionError('WebSocket connection error');
        toastError('Connection Error', 'Lost connection to server');
      };

      wsRef.current.onclose = (event) => {
        setIsConnected(false);
        wsRef.current = null;

        // Handle different close codes
        if (event.code === 1008) {
          // Authentication failed
          setConnectionError('WebSocket authentication failed');
          toastError('Authentication Error', 'WebSocket authentication failed. Please check your API key.');
          return;
        }

        // Only attempt to reconnect if we're still authenticated
        if (authState.isAuthenticated && authState.apiKey && !reconnectTimeoutRef.current) {
          setConnectionError('Connection lost, reconnecting...');
          reconnectTimeoutRef.current = window.setTimeout(() => {
            reconnectTimeoutRef.current = null;
            connect();
          }, 5000);
        }
      };
    } catch (err) {
      console.error('Failed to connect WebSocket:', err);
      setConnectionError('Failed to establish WebSocket connection');
      toastError('Connection Failed', 'Unable to establish real-time connection');
    }
  }, [url, authState.isAuthenticated, authState.apiKey, info, toastError]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      window.clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setIsConnected(false);
  }, []);

  const sendMessage = useCallback((message: any) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket not connected, cannot send message');
    }
  }, []);

  const subscribe = useCallback((type: string, handler: (payload: any) => void) => {
    if (!subscribersRef.current.has(type)) {
      subscribersRef.current.set(type, new Set());
    }
    subscribersRef.current.get(type)!.add(handler);

    // Return unsubscribe function
    return () => {
      const handlers = subscribersRef.current.get(type);
      if (handlers) {
        handlers.delete(handler);
        if (handlers.size === 0) {
          subscribersRef.current.delete(type);
        }
      }
    };
  }, []);

  const reconnect = useCallback(() => {
    disconnect();
    connect();
  }, [connect, disconnect]);

  // Connect/disconnect based on authentication status
  useEffect(() => {
    if (authState.isAuthenticated && authState.apiKey) {
      connect();
    } else {
      disconnect();
      setConnectionError(null);
    }
    
    return disconnect;
  }, [authState.isAuthenticated, authState.apiKey, connect, disconnect]);

  return (
    <WebSocketContext.Provider
      value={{
        isConnected,
        lastMessage,
        sendMessage,
        subscribe,
        reconnect,
        connectionError,
      }}
    >
      {children}
    </WebSocketContext.Provider>
  );
};

// Custom hooks for specific message types
export const useMovieUpdates = (handler: (payload: any) => void) => {
  const { subscribe } = useWebSocket();
  
  useEffect(() => {
    const unsubMovie = subscribe('movie_updated', handler);
    const unsubAdd = subscribe('movie_added', handler);
    const unsubDelete = subscribe('movie_deleted', handler);
    
    return () => {
      unsubMovie();
      unsubAdd();
      unsubDelete();
    };
  }, [subscribe, handler]);
};

export const useDownloadUpdates = (handler: (payload: any) => void) => {
  const { subscribe } = useWebSocket();
  
  useEffect(() => {
    const unsubStart = subscribe('download_started', handler);
    const unsubProgress = subscribe('download_progress', handler);
    const unsubComplete = subscribe('download_completed', handler);
    const unsubFailed = subscribe('download_failed', handler);
    
    return () => {
      unsubStart();
      unsubProgress();
      unsubComplete();
      unsubFailed();
    };
  }, [subscribe, handler]);
};

export const useQueueUpdates = (handler: (payload: any) => void) => {
  const { subscribe } = useWebSocket();
  
  useEffect(() => {
    return subscribe('queue_update', handler);
  }, [subscribe, handler]);
};