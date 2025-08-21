import React from 'react';
import { Outlet } from 'react-router-dom';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { useUI } from '../../contexts/UIContext';
import { LoadingOverlay } from '../ui/Loading';

export const Layout: React.FC = () => {
  const { sidebarOpen, isMobile, isLoading, loadingMessage, setSidebarOpen } = useUI();

  return (
    <div className="h-screen flex bg-gray-50 dark:bg-secondary-900">
      {/* Mobile sidebar overlay */}
      {isMobile && sidebarOpen && (
        <div 
          className="fixed inset-0 bg-black/50 backdrop-blur-sm z-40 md:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      {/* Sidebar */}
      <div className={`
        ${isMobile ? 'fixed' : 'relative'} 
        ${isMobile && !sidebarOpen ? '-translate-x-full' : 'translate-x-0'}
        transition-transform duration-300 ease-in-out z-50
      `}>
        <Sidebar />
      </div>
      
      {/* Main content area */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <Header />
        
        {/* Page content */}
        <main className="flex-1 overflow-y-auto relative">
          <LoadingOverlay loading={isLoading} message={loadingMessage}>
            <div className="h-full">
              <Outlet />
            </div>
          </LoadingOverlay>
        </main>
      </div>
    </div>
  );
};