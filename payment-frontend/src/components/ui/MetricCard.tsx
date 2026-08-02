import { type ElementType } from 'react';
import { TrendingUp, TrendingDown } from 'lucide-react';
import { clsx } from 'clsx';

interface MetricCardProps {
  title: string;
  value: string | number;
  change?: number;
  icon: ElementType;
  color?: 'primary' | 'success' | 'warning' | 'danger' | 'info';
}

const colorClasses = {
  primary: 'bg-primary-50 text-primary-600',
  success: 'bg-success-50 text-success-600',
  warning: 'bg-warning-50 text-warning-600',
  danger: 'bg-danger-50 text-danger-600',
  info: 'bg-primary-50 text-primary-600',
};

export function MetricCard({
  title,
  value,
  change,
  icon: Icon,
  color = 'primary',
}: MetricCardProps) {
  const hasChange = change !== undefined && change !== 0;
  const isPositive = change !== undefined && change > 0;

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm font-medium text-gray-500">{title}</p>
          <p className="text-2xl font-bold text-gray-900 mt-2">{value}</p>
          {hasChange && (
            <div
              className={clsx(
                'flex items-center gap-1 mt-2 text-sm font-medium',
                isPositive ? 'text-success-600' : 'text-danger-600'
              )}
            >
              {isPositive ? (
                <TrendingUp className="w-4 h-4" />
              ) : (
                <TrendingDown className="w-4 h-4" />
              )}
              <span>
                {isPositive ? '+' : ''}
                {change}%
              </span>
              <span className="text-gray-400 font-normal">vs last period</span>
            </div>
          )}
        </div>
        <div className={clsx('p-3 rounded-lg', colorClasses[color])}>
          <Icon className="w-6 h-6" />
        </div>
      </div>
    </div>
  );
}
