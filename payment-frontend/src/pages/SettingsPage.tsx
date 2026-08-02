import { useState, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { User, Building2, Bell, Shield, CreditCard, Save, Loader2, RefreshCw } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api, ApiError } from '@/services/api';
import { useAppStore } from '@/store';
import { clsx } from 'clsx';
import toast from 'react-hot-toast';

const tabs = [
  { id: 'profile', label: 'Profile', icon: User },
  { id: 'organization', label: 'Organization', icon: Building2 },
  { id: 'notifications', label: 'Notifications', icon: Bell },
  { id: 'security', label: 'Security', icon: Shield },
  { id: 'billing', label: 'Billing', icon: CreditCard },
];

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState('profile');
  const { user } = useAppStore();
  const queryClient = useQueryClient();

  // Fetch organization settings
  const {
    data: orgSettings,
    isLoading: orgLoading,
  } = useQuery({
    queryKey: ['organization-settings'],
    queryFn: () => api.getOrganizationSettings(),
    retry: 2,
  });

  // Fetch subscription
  const {
    data: subscription,
    isLoading: subLoading,
  } = useQuery({
    queryKey: ['subscription'],
    queryFn: () => api.getSubscription(),
    retry: 2,
  });

  // Update organization mutation
  const updateOrgMutation = useMutation({
    mutationFn: api.updateOrganizationSettings,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['organization-settings'] });
      toast.success('Organization settings updated');
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to update settings');
    },
  });

  // Form state for organization
  const [orgName, setOrgName] = useState('');
  const [orgEmail, setOrgEmail] = useState('');
  const [orgTimezone, setOrgTimezone] = useState('UTC');
  const [orgCurrency, setOrgCurrency] = useState('USD');

  // Initialize form with data
  useEffect(() => {
    if (orgSettings) {
      setOrgName(orgSettings.name || '');
      setOrgEmail(orgSettings.billing_email || '');
      setOrgTimezone(orgSettings.timezone || 'UTC');
      setOrgCurrency(orgSettings.default_currency || 'USD');
    }
  }, [orgSettings]);

  const handleSaveOrganization = () => {
    updateOrgMutation.mutate({
      name: orgName,
      billing_email: orgEmail,
      timezone: orgTimezone,
      default_currency: orgCurrency,
    });
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Settings</h1>
        <p className="text-gray-500">Manage your account and preferences</p>
      </div>

      <div className="flex flex-col lg:flex-row gap-6">
        {/* Sidebar Navigation */}
        <div className="w-full lg:w-64 flex-shrink-0">
          <Card padding={false}>
            <nav className="p-2">
              {tabs.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={clsx(
                    'flex items-center gap-3 w-full px-3 py-2.5 rounded-lg text-sm font-medium transition-colors',
                    activeTab === tab.id
                      ? 'bg-primary-50 text-primary-700'
                      : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900'
                  )}
                >
                  <tab.icon className="w-5 h-5" />
                  {tab.label}
                </button>
              ))}
            </nav>
          </Card>
        </div>

        {/* Main Content */}
        <div className="flex-1">
          {/* Profile Tab */}
          {activeTab === 'profile' && (
            <Card>
              <div className="mb-6">
                <h2 className="text-lg font-semibold text-gray-900">Profile Settings</h2>
                <p className="text-sm text-gray-500">Update your personal information</p>
              </div>
              <div className="space-y-6">
                <div className="flex items-center gap-6">
                  <div className="w-20 h-20 bg-primary-100 rounded-full flex items-center justify-center">
                    <span className="text-2xl font-bold text-primary-700">
                      {user?.name?.charAt(0) || 'U'}
                    </span>
                  </div>
                  <div>
                    <button className="btn-secondary text-sm">Change Avatar</button>
                    <p className="text-xs text-gray-500 mt-1">JPG, PNG or GIF. Max 2MB.</p>
                  </div>
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Full Name</label>
                    <input
                      type="text"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      defaultValue={user?.name || ''}
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Email Address</label>
                    <input
                      type="email"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      defaultValue={user?.email || ''}
                      disabled
                    />
                    <p className="text-xs text-gray-500 mt-1">Contact support to change your email</p>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Phone Number</label>
                    <input
                      type="tel"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      placeholder="+1 234 567 890"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Timezone</label>
                    <select className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
                      <option>UTC</option>
                      <option>America/New_York</option>
                      <option>Europe/London</option>
                      <option>Asia/Dubai</option>
                      <option>Asia/Kolkata</option>
                    </select>
                  </div>
                </div>
                <div className="flex justify-end">
                  <button className="btn-primary flex items-center gap-2">
                    <Save className="w-4 h-4" />
                    Save Changes
                  </button>
                </div>
              </div>
            </Card>
          )}

          {/* Organization Tab */}
          {activeTab === 'organization' && (
            <Card>
              <div className="mb-6">
                <h2 className="text-lg font-semibold text-gray-900">Organization Settings</h2>
                <p className="text-sm text-gray-500">Manage your organization details</p>
              </div>
              {orgLoading ? (
                <div className="flex items-center justify-center py-12">
                  <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
                </div>
              ) : (
                <div className="space-y-6">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Organization Name</label>
                    <input
                      type="text"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      value={orgName}
                      onChange={(e) => setOrgName(e.target.value)}
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Billing Email</label>
                    <input
                      type="email"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      value={orgEmail}
                      onChange={(e) => setOrgEmail(e.target.value)}
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-6">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">Timezone</label>
                      <select
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                        value={orgTimezone}
                        onChange={(e) => setOrgTimezone(e.target.value)}
                      >
                        <option value="UTC">UTC</option>
                        <option value="America/New_York">America/New_York</option>
                        <option value="Europe/London">Europe/London</option>
                        <option value="Asia/Dubai">Asia/Dubai</option>
                        <option value="Asia/Kolkata">Asia/Kolkata</option>
                      </select>
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">Default Currency</label>
                      <select
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                        value={orgCurrency}
                        onChange={(e) => setOrgCurrency(e.target.value)}
                      >
                        <option value="USD">USD - US Dollar</option>
                        <option value="AED">AED - UAE Dirham</option>
                        <option value="EUR">EUR - Euro</option>
                        <option value="GBP">GBP - British Pound</option>
                        <option value="INR">INR - Indian Rupee</option>
                      </select>
                    </div>
                  </div>
                  <div className="flex justify-end">
                    <button
                      className="btn-primary flex items-center gap-2"
                      onClick={handleSaveOrganization}
                      disabled={updateOrgMutation.isPending}
                    >
                      {updateOrgMutation.isPending ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <Save className="w-4 h-4" />
                      )}
                      Save Changes
                    </button>
                  </div>
                </div>
              )}
            </Card>
          )}

          {/* Notifications Tab */}
          {activeTab === 'notifications' && (
            <Card>
              <div className="mb-6">
                <h2 className="text-lg font-semibold text-gray-900">Notification Preferences</h2>
                <p className="text-sm text-gray-500">Configure how you receive notifications</p>
              </div>
              <div className="space-y-6">
                {[
                  { id: 'transactions', label: 'Transaction Updates', description: 'Get notified about payment status changes' },
                  { id: 'failures', label: 'Payment Failures', description: 'Alert when payments fail or are declined' },
                  { id: 'gateways', label: 'Gateway Health', description: 'Notifications about gateway connectivity issues' },
                  { id: 'security', label: 'Security Alerts', description: 'Important security notifications' },
                  { id: 'billing', label: 'Billing Updates', description: 'Invoice and subscription notifications' },
                ].map((item) => (
                  <div key={item.id} className="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
                    <div>
                      <h3 className="font-medium text-gray-900">{item.label}</h3>
                      <p className="text-sm text-gray-500">{item.description}</p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input type="checkbox" className="sr-only peer" defaultChecked />
                      <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
                    </label>
                  </div>
                ))}
                <div className="flex justify-end">
                  <button className="btn-primary flex items-center gap-2">
                    <Save className="w-4 h-4" />
                    Save Preferences
                  </button>
                </div>
              </div>
            </Card>
          )}

          {/* Security Tab */}
          {activeTab === 'security' && (
            <Card>
              <div className="mb-6">
                <h2 className="text-lg font-semibold text-gray-900">Security Settings</h2>
                <p className="text-sm text-gray-500">Manage your security preferences</p>
              </div>
              <div className="space-y-6">
                <div className="p-4 bg-gray-50 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="font-medium text-gray-900">Two-Factor Authentication</h3>
                      <p className="text-sm text-gray-500">Add an extra layer of security to your account</p>
                    </div>
                    <button className="btn-secondary">Enable 2FA</button>
                  </div>
                </div>
                <div className="p-4 bg-gray-50 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="font-medium text-gray-900">Session Management</h3>
                      <p className="text-sm text-gray-500">View and manage active sessions</p>
                    </div>
                    <button className="btn-secondary">View Sessions</button>
                  </div>
                </div>
                <div className="p-4 bg-danger-50 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="font-medium text-danger-900">Delete Account</h3>
                      <p className="text-sm text-danger-700">
                        Permanently delete your account and all associated data
                      </p>
                    </div>
                    <button className="btn-danger">Delete Account</button>
                  </div>
                </div>
              </div>
            </Card>
          )}

          {/* Billing Tab */}
          {activeTab === 'billing' && (
            <Card>
              <div className="mb-6">
                <h2 className="text-lg font-semibold text-gray-900">Billing</h2>
                <p className="text-sm text-gray-500">Manage your subscription and payment methods</p>
              </div>
              {subLoading ? (
                <div className="flex items-center justify-center py-12">
                  <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
                </div>
              ) : (
                <div className="space-y-6">
                  {/* Current Plan */}
                  <div className="p-6 bg-primary-50 rounded-lg">
                    <div className="flex items-center justify-between">
                      <div>
                        <h3 className="text-lg font-semibold text-gray-900">
                          Current Plan: {subscription?.plan.name || 'Free'}
                        </h3>
                        <p className="text-gray-600">
                          {subscription?.plan.price_monthly 
                            ? `$${subscription.plan.price_monthly}/month`
                            : 'Free'} 
                          • {subscription?.usage.included || 100} transactions included
                        </p>
                      </div>
                      <button className="btn-primary">Upgrade Plan</button>
                    </div>
                  </div>

                  {/* Usage */}
                  {subscription?.usage && (
                    <div>
                      <h3 className="font-medium text-gray-900 mb-4">Usage This Period</h3>
                      <div className="grid grid-cols-3 gap-4">
                        <div className="text-center p-4 bg-gray-50 rounded-lg">
                          <p className="text-2xl font-bold text-gray-900">
                            {subscription.usage.transactions.toLocaleString()}
                          </p>
                          <p className="text-sm text-gray-500">Transactions</p>
                          <p className="text-xs text-gray-400 mt-1">
                            of {subscription.usage.included.toLocaleString()} included
                          </p>
                        </div>
                        <div className="text-center p-4 bg-gray-50 rounded-lg">
                          <p className="text-2xl font-bold text-gray-900">
                            {subscription.usage.overage > 0 ? subscription.usage.overage : 0}
                          </p>
                          <p className="text-sm text-gray-500">Overage</p>
                          {subscription.usage.overage > 0 && (
                            <p className="text-xs text-warning-600 mt-1">Extra charges apply</p>
                          )}
                        </div>
                        <div className="text-center p-4 bg-gray-50 rounded-lg">
                          <p className="text-2xl font-bold text-gray-900">
                            ${(subscription.usage.transactions * 0.001).toFixed(2)}
                          </p>
                          <p className="text-sm text-gray-500">Orchestration Fees</p>
                          <p className="text-xs text-gray-400 mt-1">$0.001 per transaction</p>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Payment Method */}
                  <div>
                    <h3 className="font-medium text-gray-900 mb-4">Payment Method</h3>
                    <div className="flex items-center gap-4 p-4 bg-gray-50 rounded-lg">
                      <div className="w-12 h-8 bg-gray-200 rounded flex items-center justify-center">
                        <span className="text-xs font-bold text-gray-600">VISA</span>
                      </div>
                      <div>
                        <p className="font-medium text-gray-900">•••• •••• •••• 4242</p>
                        <p className="text-sm text-gray-500">Expires 12/2025</p>
                      </div>
                      <button className="ml-auto btn-secondary text-sm">Update</button>
                    </div>
                  </div>

                  {/* Billing History */}
                  <div>
                    <h3 className="font-medium text-gray-900 mb-4">Billing History</h3>
                    <div className="text-center py-8 text-gray-500">
                      No billing history yet
                    </div>
                  </div>
                </div>
              )}
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
