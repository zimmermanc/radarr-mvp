import React from 'react';
import { BrowserRouter } from 'react-router-dom';
import { render } from '@testing-library/react';
import { ThemeProvider } from '../contexts/ThemeContext';
import { UIProvider } from '../contexts/UIContext';
import { AuthProvider } from '../contexts/AuthContext';
import { ToastProvider } from '../components/ui/Toast';
import { ConfirmDialogProvider } from '../components/ui/ConfirmDialog';
import { WebSocketProvider } from '../contexts/WebSocketContext';
import { ErrorBoundary } from '../components/ErrorBoundary';

interface TestWrapperProps {
  children: React.ReactNode;
  initialRoute?: string;
}

/**
 * Comprehensive test wrapper that provides all necessary React contexts
 * for testing components that depend on global state and providers.
 */
export const TestWrapper: React.FC<TestWrapperProps> = ({ 
  children, 
  initialRoute = '/' 
}) => {
  return (
    <ErrorBoundary>
      <BrowserRouter>
        <ThemeProvider>
          <UIProvider>
            <ToastProvider>
              <ConfirmDialogProvider>
                <AuthProvider>
                  <WebSocketProvider>
                    <div data-testid="test-wrapper">
                      {children}
                    </div>
                  </WebSocketProvider>
                </AuthProvider>
              </ConfirmDialogProvider>
            </ToastProvider>
          </UIProvider>
        </ThemeProvider>
      </BrowserRouter>
    </ErrorBoundary>
  );
};

/**
 * Minimal test wrapper for components that don't need full context
 */
export const MinimalTestWrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return (
    <BrowserRouter>
      <div data-testid="minimal-test-wrapper">
        {children}
      </div>
    </BrowserRouter>
  );
};

/**
 * Custom render function that includes all providers by default
 */
export const renderWithProviders = (
  ui: React.ReactElement,
  options: { 
    initialRoute?: string;
    wrapper?: React.ComponentType<{ children: React.ReactNode }>;
  } = {}
) => {
  const { initialRoute = '/', wrapper } = options;
  
  const Wrapper = wrapper || ((props: { children: React.ReactNode }) => (
    <TestWrapper initialRoute={initialRoute}>
      {props.children}
    </TestWrapper>
  ));

  return render(ui, { wrapper: Wrapper });
};