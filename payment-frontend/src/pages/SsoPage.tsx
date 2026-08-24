import { useState } from 'react';
import { 
  Shield, 
  Plus, 
  Trash2, 
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import toast from 'react-hot-toast';

interface SsoProvider {
  id: string;
  name: string;
  type: 'saml' | 'oidc';
  status: 'active' | 'inactive';
  created_at: string;
  metadata_url?: string;
  client_id?: string;
}

export function SsoPage() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [providerType, setProviderType] = useState<'saml' | 'oidc'>('oidc');
  const [providerName, setProviderName] = useState('');
  const [metadataUrl, setMetadataUrl] = useState('');
  const [clientId, setClientId] = useState('');
  

  // Mock SSO providers for demonstration
  const [providers, setProviders] = useState<SsoProvider[]>([
    {
      id: 'sso_001',
      name: 'Google Workspace',
      type: 'oidc',
      status: 'active',
      created_at: '2024-01-15T10:30:00Z',
      client_id: 'google-client-id-123',
    },
    {
      id: 'sso_002',
      name: 'Okta',
      type: 'saml',
      status: 'inactive',
      created_at: '2024-02-01T14:00:00Z',
      metadata_url: 'https://company.okta.com/metadata',
    },
  ]);

  const handleCreateProvider = () => {
    if (!providerName.trim()) {
      toast.error('Please enter a provider name');
      return;
    }
    
    const newProvider: SsoProvider = {
      id: `sso_${Date.now()}`,
      name: providerName,
      type: providerType,
      status: 'active',
      created_at: new Date().toISOString(),
      ...(providerType === 'oidc' ? { client_id: clientId } : { metadata_url: metadataUrl }),
    };
    
    setProviders([...providers, newProvider]);
    setShowCreateModal(false);
    setProviderName('');
    setMetadataUrl('');
    setClientId('');
    toast.success('SSO provider created');
  };

  const handleDeleteProvider = (id: string) => {
    if (window.confirm('Are you sure you want to delete this SSO provider?')) {
      setProviders(providers.filter((p) => p.id !== id));
      toast.success('SSO provider deleted');
    }
  };

  const handleToggleProvider = (id: string) => {
    setProviders(
      providers.map((p) =>
        p.id === id ? { ...p, status: p.status === 'active' ? 'inactive' : 'active' } : p
      )
    );
    toast.success('Provider status updated');
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Enterprise SSO</h1>
          <p className="text-gray-500">Configure Single Sign-On for your organization</p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="btn-primary flex items-center gap-2"
        >
          <Plus className="w-4 h-4" />
          Add Provider
        </button>
      </div>

      {/* SSO Info */}
      <Card className="border-primary-200 bg-primary-50">
        <div className="flex items-start gap-3">
          <Shield className="w-5 h-5 text-primary-600 mt-0.5" />
          <div>
            <h3 className="font-medium text-primary-800">Enterprise Feature</h3>
            <p className="text-sm text-primary-700 mt-1">
              SSO is available on the Enterprise plan. Contact sales to enable this feature for your organization.
            </p>
          </div>
        </div>
      </Card>

      {/* SSO Providers */}
      <Card>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">SSO Providers</h2>
        </div>

        {providers.length > 0 ? (
          <div className="space-y-4">
            {providers.map((provider) => (
              <div
                key={provider.id}
                className="flex items-center justify-between p-4 bg-gray-50 rounded-lg"
              >
                <div className="flex items-center gap-4">
                  <div className="w-10 h-10 bg-white rounded-lg border border-gray-200 flex items-center justify-center">
                    <Shield className="w-5 h-5 text-gray-600" />
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <h3 className="font-medium text-gray-900">{provider.name}</h3>
                      <span className={`px-2 py-0.5 text-xs font-medium rounded ${
                        provider.status === 'active'
                          ? 'bg-success-100 text-success-700'
                          : 'bg-gray-100 text-gray-700'
                      }`}>
                        {provider.status}
                      </span>
                      <span className="px-2 py-0.5 text-xs font-medium bg-primary-100 text-primary-700 rounded uppercase">
                        {provider.type}
                      </span>
                    </div>
                    <p className="text-sm text-gray-500 mt-1">
                      Created {new Date(provider.created_at).toLocaleDateString()}
                    </p>
                    {provider.client_id && (
                      <p className="text-xs text-gray-400 mt-1">
                        Client ID: {provider.client_id.slice(0, 20)}...
                      </p>
                    )}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => handleToggleProvider(provider.id)}
                    className={`px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                      provider.status === 'active'
                        ? 'text-warning-600 hover:bg-warning-50'
                        : 'text-success-600 hover:bg-success-50'
                    }`}
                  >
                    {provider.status === 'active' ? 'Disable' : 'Enable'}
                  </button>
                  <button
                    onClick={() => handleDeleteProvider(provider.id)}
                    className="p-2 text-gray-400 hover:text-danger-600 hover:bg-danger-50 rounded-lg transition-colors"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-12">
            <Shield className="w-12 h-12 text-gray-300 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-gray-900">No SSO providers configured</h3>
            <p className="text-gray-500 mt-1">
              Add an SSO provider to enable Single Sign-On for your organization
            </p>
          </div>
        )}
      </Card>

      {/* Supported Providers */}
      <Card>
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Supported Providers</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {[
            { name: 'Google Workspace', type: 'oidc', description: 'Google Workspace SSO' },
            { name: 'Okta', type: 'saml', description: 'Okta workforce SSO' },
            { name: 'Azure AD', type: 'oidc', description: 'Microsoft Entra ID' },
            { name: 'OneLogin', type: 'saml', description: 'OneLogin SSO' },
            { name: 'Auth0', type: 'oidc', description: 'Auth0 enterprise SSO' },
            { name: 'Custom SAML', type: 'saml', description: 'Any SAML 2.0 provider' },
          ].map((provider) => (
            <div
              key={provider.name}
              className="p-4 border border-gray-200 rounded-lg hover:border-primary-300 transition-colors"
            >
              <div className="flex items-center gap-2 mb-2">
                <Shield className="w-4 h-4 text-gray-600" />
                <h3 className="font-medium text-gray-900">{provider.name}</h3>
              </div>
              <p className="text-sm text-gray-500">{provider.description}</p>
              <span className="inline-block mt-2 px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-600 rounded uppercase">
                {provider.type}
              </span>
            </div>
          ))}
        </div>
      </Card>

      {/* Create Provider Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <Card className="w-full max-w-lg">
            <div className="p-6">
              <h2 className="text-xl font-semibold text-gray-900 mb-4">Add SSO Provider</h2>
              
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Provider Name
                  </label>
                  <input
                    type="text"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                    placeholder="e.g., Google Workspace"
                    value={providerName}
                    onChange={(e) => setProviderName(e.target.value)}
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Protocol
                  </label>
                  <div className="flex gap-4">
                    <label className="flex items-center gap-2">
                      <input
                        type="radio"
                        name="protocol"
                        value="oidc"
                        checked={providerType === 'oidc'}
                        onChange={() => setProviderType('oidc')}
                        className="text-primary-600 focus:ring-primary-500"
                      />
                      <span className="text-sm text-gray-700">OpenID Connect (OIDC)</span>
                    </label>
                    <label className="flex items-center gap-2">
                      <input
                        type="radio"
                        name="protocol"
                        value="saml"
                        checked={providerType === 'saml'}
                        onChange={() => setProviderType('saml')}
                        className="text-primary-600 focus:ring-primary-500"
                      />
                      <span className="text-sm text-gray-700">SAML 2.0</span>
                    </label>
                  </div>
                </div>

                {providerType === 'oidc' ? (
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Client ID
                    </label>
                    <input
                      type="text"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      placeholder="Enter your OIDC client ID"
                      value={clientId}
                      onChange={(e) => setClientId(e.target.value)}
                    />
                  </div>
                ) : (
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Metadata URL
                    </label>
                    <input
                      type="url"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      placeholder="https://idp.example.com/metadata"
                      value={metadataUrl}
                      onChange={(e) => setMetadataUrl(e.target.value)}
                    />
                  </div>
                )}

                <div className="flex gap-3 mt-6">
                  <button
                    className="btn-secondary flex-1"
                    onClick={() => setShowCreateModal(false)}
                  >
                    Cancel
                  </button>
                  <button
                    className="btn-primary flex-1"
                    onClick={handleCreateProvider}
                  >
                    Create Provider
                  </button>
                </div>
              </div>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
