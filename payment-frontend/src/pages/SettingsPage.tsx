import { useState } from 'react';
import { User, Building2, Bell, Shield, CreditCard, Save } from 'lucide-react';
import { Card, CardHeader } from '@/components/ui/Card';
import { clsx } from 'clsx';

const tabs = [
  { id: 'profile', label: 'Profile', icon: User },
  { id: 'organization', label: 'Organization', icon: Building2 },
  { id: 'notifications', label: 'Notifications', icon: Bell },
  { id: 'security', label: 'Security', icon: Shield },
  { id: 'billing', label: 'Billing', icon: CreditCard },
];

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState('profile');

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
          {activeTab === 'profile' && (
            <Card>
              <CardHeader title="Profile Settings" description="Update your personal information" />
              <div className="space-y-6">
                <div className="flex items-center gap-6">
                  <div className="w-20 h-20 bg-primary-100 rounded-full flex items-center justify-center">
                    <span className="text-2xl font-bold text-primary-700">JD</span>
                  </div>
                  <div>
                    <button className="btn-secondary text-sm">Change Avatar</button>
                    <p className="text-xs text-gray-500 mt-1">JPG, PNG or GIF. Max 2MB.</p>
                  </div>
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <div>
                    <label className="label">Full Name</label>
                    <input type="text" className="input" defaultValue="John Doe" />
                  </div>
                  <div>
                    <label className="label">Email Address</label>
                    <input type="email" className="input" defaultValue="john@example.com" />
                  </div>
                  <div>
                    <label className="label">Phone Number</label>
                    <input type="tel" className="input" defaultValue="+1 234 567 890" />
                  </div>
                  <div>
                    <label className="label">Timezone</label>
                    <select className="input">
                      <option>UTC</option>
                      <option>America/New_York</option>
                      <option>Europe/London</option>
                      <option>Asia/Dubai</option>
                      <option>Asia/Kolkata</option>
                    </select>
                  </div>
                </div>
                <div className="flex justify-end">
                  <button className="btn-primary">
                    <Save className="w-4 h-4 mr-2" />
                    Save Changes
                  </button>
                </div>
              </div>
            </Card>
          )}

          {activeTab === 'organization' && (
            <Card>
              <CardHeader title="Organization Settings" description="Manage your organization details" />
              <div className="space-y-6">
                <div>
                  <label className="label">Organization Name</label>
                  <input type="text" className="input" defaultValue="Acme Inc" />
                </div>
                <div>
                  <label className="label">Industry</label>
                  <select className="input">
                    <option>E-commerce</option>
                    <option>SaaS</option>
                    <option>Marketplace</option>
                    <option>Travel</option>
                    <option>Gaming</option>
                    <option>Other</option>
                  </select>
                </div>
                <div>
                  <label className="label">Website</label>
                  <input type="url" className="input" defaultValue="https://acme.com" />
                </div>
                <div>
                  <label className="label">Country</label>
                  <select className="input">
                    <option>United States</option>
                    <option>United Arab Emirates</option>
                    <option>India</option>
                    <option>United Kingdom</option>
                    <option>Germany</option>
                  </select>
                </div>
                <div className="flex justify-end">
                  <button className="btn-primary">
                    <Save className="w-4 h-4 mr-2" />
                    Save Changes
                  </button>
                </div>
              </div>
            </Card>
          )}

          {activeTab === 'notifications' && (
            <Card>
              <CardHeader title="Notification Preferences" description="Configure how you receive notifications" />
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
                  <button className="btn-primary">
                    <Save className="w-4 h-4 mr-2" />
                    Save Preferences
                  </button>
                </div>
              </div>
            </Card>
          )}

          {activeTab === 'security' && (
            <Card>
              <CardHeader title="Security Settings" description="Manage your security preferences" />
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

          {activeTab === 'billing' && (
            <Card>
              <CardHeader title="Billing" description="Manage your subscription and payment methods" />
              <div className="space-y-6">
                <div className="p-6 bg-primary-50 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-lg font-semibold text-gray-900">Current Plan: Growth</h3>
                      <p className="text-gray-600">$499/month • 10,000 transactions included</p>
                    </div>
                    <button className="btn-primary">Upgrade Plan</button>
                  </div>
                </div>
                <div>
                  <h3 className="font-medium text-gray-900 mb-4">Usage This Period</h3>
                  <div className="grid grid-cols-3 gap-4">
                    <div className="text-center p-4 bg-gray-50 rounded-lg">
                      <p className="text-2xl font-bold text-gray-900">7,542</p>
                      <p className="text-sm text-gray-500">Transactions</p>
                    </div>
                    <div className="text-center p-4 bg-gray-50 rounded-lg">
                      <p className="text-2xl font-bold text-gray-900">$754,200</p>
                      <p className="text-sm text-gray-500">Volume</p>
                    </div>
                    <div className="text-center p-4 bg-gray-50 rounded-lg">
                      <p className="text-2xl font-bold text-gray-900">$37.71</p>
                      <p className="text-sm text-gray-500">Orchestration Fees</p>
                    </div>
                  </div>
                </div>
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
              </div>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
