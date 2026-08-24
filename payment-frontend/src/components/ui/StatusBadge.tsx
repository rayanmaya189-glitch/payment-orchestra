import { clsx } from 'clsx';

type BadgeStatus = 
  | 'created'
  | 'authorized'
  | 'captured'
  | 'pending'
  | 'processing'
  | 'failed'
  | 'refunded'
  | 'voided'
  | 'active'
  | 'inactive'
  | 'error'
  | 'testing'
  | 'success'
  | 'warning';

interface StatusBadgeProps {
  status: BadgeStatus | string;
  size?: 'sm' | 'md';
}

const statusStyles: Record<string, string> = {
  // Payment statuses
  created: 'bg-gray-100 text-gray-700',
  pending: 'bg-warning-50 text-warning-700',
  processing: 'bg-primary-50 text-primary-700',
  authorized: 'bg-primary-50 text-primary-700',
  captured: 'bg-success-50 text-success-700',
  success: 'bg-success-50 text-success-700',
  failed: 'bg-danger-50 text-danger-700',
  error: 'bg-danger-50 text-danger-700',
  refunded: 'bg-warning-50 text-warning-700',
  voided: 'bg-gray-100 text-gray-700',
  
  // Gateway statuses
  active: 'bg-success-50 text-success-700',
  inactive: 'bg-gray-100 text-gray-700',
  testing: 'bg-warning-50 text-warning-700',
  warning: 'bg-warning-50 text-warning-700',
};

const dotStyles: Record<string, string> = {
  created: 'bg-gray-500',
  pending: 'bg-warning-500',
  processing: 'bg-primary-500',
  authorized: 'bg-primary-500',
  captured: 'bg-success-500',
  success: 'bg-success-500',
  failed: 'bg-danger-500',
  error: 'bg-danger-500',
  refunded: 'bg-warning-500',
  voided: 'bg-gray-500',
  active: 'bg-success-500',
  inactive: 'bg-gray-500',
  testing: 'bg-warning-500',
  warning: 'bg-warning-500',
};

export function StatusBadge({ status, size = 'sm' }: StatusBadgeProps) {
  const normalizedStatus = status.toLowerCase();
  const style = statusStyles[normalizedStatus] || 'bg-gray-100 text-gray-700';
  const dotStyle = dotStyles[normalizedStatus] || 'bg-gray-500';

  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1.5 rounded-full font-medium',
        style,
        size === 'sm' ? 'px-2.5 py-0.5 text-xs' : 'px-3 py-1 text-sm'
      )}
    >
      <span className={clsx('w-1.5 h-1.5 rounded-full', dotStyle)} />
      <span className="capitalize">{status}</span>
    </span>
  );
}
