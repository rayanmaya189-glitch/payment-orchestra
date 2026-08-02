import { NavLink } from 'react-router-dom';
import {
  LayoutDashboard,
  CreditCard,
  GitBranch,
  Plug,
  Key,
  BarChart3,
  Settings,
  HelpCircle,
  ChevronLeft,
  ChevronRight,
  Zap,
  Code,
  Webhook,
  RefreshCw,
  Shield,
  Activity,
  Sparkles,
  X,
  Store,
} from 'lucide-react';
import { useAppStore } from '@/store';
import { clsx } from 'clsx';

const navigation = [
  { name: 'Dashboard', href: '/', icon: LayoutDashboard },
  { name: 'Payments', href: '/payments', icon: CreditCard },
  { name: 'Routing', href: '/routing', icon: GitBranch },
  { name: 'Connectors', href: '/connectors', icon: Plug },
  { name: 'Marketplace', href: '/marketplace', icon: Store },
  { name: 'API Keys', href: '/api-keys', icon: Key },
  { name: 'Analytics', href: '/analytics', icon: BarChart3 },
  { name: 'Reconciliation', href: '/reconciliation', icon: RefreshCw },
  { name: 'Audit Logs', href: '/audit-logs', icon: Shield },
  { name: 'Rate Limits', href: '/rate-limits', icon: Activity },
  { name: 'AI Assistant', href: '/assistant', icon: Sparkles },
  { name: 'SSO', href: '/sso', icon: Shield },
  { name: 'Webhooks', href: '/webhooks', icon: Webhook },
  { name: 'Developer', href: '/developer', icon: Code },
  { name: 'Settings', href: '/settings', icon: Settings },
];

const secondaryNavigation = [
  { name: 'Documentation', href: '/docs', icon: HelpCircle, external: true },
];

export function Sidebar() {
  const { sidebarOpen, toggleSidebar, setSidebarOpen, organization } = useAppStore();

  const handleNavClick = () => {
    // Close sidebar on mobile when navigating
    if (window.innerWidth < 1024) {
      setSidebarOpen(false);
    }
  };

  return (
    <>
      {/* Mobile sidebar */}
      <aside
        className={clsx(
          'fixed inset-y-0 left-0 z-50 bg-white border-r border-gray-200 transition-all duration-300 lg:hidden',
          sidebarOpen ? 'w-64 translate-x-0' : 'w-64 -translate-x-full'
        )}
      >
        <div className="flex flex-col h-full">
          {/* Logo */}
          <div className="flex items-center justify-between h-16 px-4 border-b border-gray-200">
            <div className="flex items-center gap-3">
              <div className="flex items-center justify-center w-10 h-10 bg-primary-600 rounded-lg">
                <Zap className="w-6 h-6 text-white" />
              </div>
              <div>
                <h1 className="text-lg font-bold text-gray-900">PaymentOrchestra</h1>
                <p className="text-xs text-gray-500 capitalize">
                  {organization?.tier || 'Starter'} Plan
                </p>
              </div>
            </div>
            <button
              onClick={toggleSidebar}
              className="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* Main Navigation */}
          <nav className="flex-1 px-3 py-4 space-y-1 overflow-y-auto">
            {navigation.map((item) => (
              <NavLink
                key={item.name}
                to={item.href}
                onClick={handleNavClick}
                className={({ isActive }) =>
                  clsx(
                    'flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors',
                    isActive
                      ? 'bg-primary-50 text-primary-700'
                      : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900'
                  )
                }
              >
                <item.icon className="w-5 h-5 flex-shrink-0" />
                <span>{item.name}</span>
              </NavLink>
            ))}
          </nav>

          {/* Secondary Navigation */}
          <div className="px-3 py-4 border-t border-gray-200 space-y-1">
            {secondaryNavigation.map((item) => (
              <a
                key={item.name}
                href={item.href}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium text-gray-600 hover:bg-gray-50 hover:text-gray-900 transition-colors"
              >
                <item.icon className="w-5 h-5 flex-shrink-0" />
                <span>{item.name}</span>
              </a>
            ))}
          </div>
        </div>
      </aside>

      {/* Desktop sidebar */}
      <aside
        className={clsx(
          'hidden lg:block fixed inset-y-0 left-0 z-50 bg-white border-r border-gray-200 transition-all duration-300',
          sidebarOpen ? 'w-64' : 'w-20'
        )}
      >
        <div className="flex flex-col h-full">
          {/* Logo */}
          <div className="flex items-center h-16 px-4 border-b border-gray-200">
            <div className="flex items-center gap-3">
              <div className="flex items-center justify-center w-10 h-10 bg-primary-600 rounded-lg">
                <Zap className="w-6 h-6 text-white" />
              </div>
              {sidebarOpen && (
                <div className="animate-fade-in">
                  <h1 className="text-lg font-bold text-gray-900">PaymentOrchestra</h1>
                  <p className="text-xs text-gray-500 capitalize">
                    {organization?.tier || 'Starter'} Plan
                  </p>
                </div>
              )}
            </div>
          </div>

          {/* Main Navigation */}
          <nav className="flex-1 px-3 py-4 space-y-1 overflow-y-auto">
            {navigation.map((item) => (
              <NavLink
                key={item.name}
                to={item.href}
                className={({ isActive }) =>
                  clsx(
                    'flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors',
                    isActive
                      ? 'bg-primary-50 text-primary-700'
                      : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900'
                  )
                }
              >
                <item.icon className="w-5 h-5 flex-shrink-0" />
                {sidebarOpen && <span>{item.name}</span>}
              </NavLink>
            ))}
          </nav>

          {/* Secondary Navigation */}
          <div className="px-3 py-4 border-t border-gray-200 space-y-1">
            {secondaryNavigation.map((item) => (
              <a
                key={item.name}
                href={item.href}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium text-gray-600 hover:bg-gray-50 hover:text-gray-900 transition-colors"
              >
                <item.icon className="w-5 h-5 flex-shrink-0" />
                {sidebarOpen && <span>{item.name}</span>}
              </a>
            ))}
          </div>

          {/* Collapse Button */}
          <div className="px-3 py-4 border-t border-gray-200">
            <button
              onClick={toggleSidebar}
              className="flex items-center justify-center w-full px-3 py-2 rounded-lg text-gray-500 hover:bg-gray-50 hover:text-gray-700 transition-colors"
            >
              {sidebarOpen ? (
                <ChevronLeft className="w-5 h-5" />
              ) : (
                <ChevronRight className="w-5 h-5" />
              )}
            </button>
          </div>
        </div>
      </aside>
    </>
  );
}
