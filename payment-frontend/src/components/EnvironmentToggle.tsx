import { useState, useEffect } from 'react';
import { Monitor, TestTube, AlertCircle } from 'lucide-react';
import { clsx } from 'clsx';
import toast from 'react-hot-toast';

interface EnvironmentToggleProps {
  onEnvironmentChange?: (environment: 'sandbox' | 'production') => void;
  className?: string;
}

export function EnvironmentToggle({ onEnvironmentChange, className }: EnvironmentToggleProps) {
  const [environment, setEnvironment] = useState<'sandbox' | 'production'>(
    () => (localStorage.getItem('environment') as 'sandbox' | 'production') || 'sandbox'
  );

  useEffect(() => {
    localStorage.setItem('environment', environment);
    onEnvironmentChange?.(environment);
  }, [environment, onEnvironmentChange]);

  const handleToggle = () => {
    const newEnv = environment === 'sandbox' ? 'production' : 'sandbox';
    
    if (newEnv === 'production') {
      // Show warning when switching to production
      const confirmed = window.confirm(
        '⚠️ You are switching to PRODUCTION mode.\n\n' +
        'All actions will affect live data and real transactions.\n\n' +
        'Are you sure you want to continue?'
      );
      if (!confirmed) return;
    }
    
    setEnvironment(newEnv);
    
    if (newEnv === 'sandbox') {
      toast.success('Switched to Sandbox mode');
    } else {
      toast.success('Switched to Production mode');
    }
  };

  return (
    <div className={clsx('flex items-center gap-2', className)}>
      <button
        onClick={handleToggle}
        className={clsx(
          'relative inline-flex h-8 w-14 items-center rounded-full transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2',
          environment === 'production'
            ? 'bg-danger-500'
            : 'bg-success-500'
        )}
        role="switch"
        aria-checked={environment === 'production'}
        aria-label={`Switch to ${environment === 'sandbox' ? 'production' : 'sandbox'} mode`}
      >
        <span
          className={clsx(
            'inline-block h-6 w-6 transform rounded-full bg-white shadow-lg transition-transform duration-200 ease-in-out',
            environment === 'production' ? 'translate-x-7' : 'translate-x-1'
          )}
        />
      </button>
      
      <div className="flex items-center gap-1.5">
        {environment === 'sandbox' ? (
          <>
            <TestTube className="w-4 h-4 text-success-600" />
            <span className="text-sm font-medium text-success-700">Sandbox</span>
          </>
        ) : (
          <>
            <Monitor className="w-4 h-4 text-danger-600" />
            <span className="text-sm font-medium text-danger-700">Production</span>
          </>
        )}
      </div>
      
      {environment === 'production' && (
        <div className="flex items-center gap-1 px-2 py-0.5 bg-danger-100 rounded text-xs text-danger-700">
          <AlertCircle className="w-3 h-3" />
          <span>Live</span>
        </div>
      )}
    </div>
  );
}
