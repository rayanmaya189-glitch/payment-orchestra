import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useMutation } from '@tanstack/react-query';
import { 
  Zap, 
  Building2, 
  Settings, 
  Check,
  ArrowRight,
  ArrowLeft,
  Plug,
  Key,
  Loader2,
  Copy,
  CheckCircle,
} from 'lucide-react';
import { clsx } from 'clsx';
import toast from 'react-hot-toast';
import { api, ApiError } from '@/services/api';
import { useAppStore } from '@/store';

const steps = [
  { id: 1, title: 'Organization', icon: Building2 },
  { id: 2, title: 'First Connector', icon: Plug },
  { id: 3, title: 'Routing Setup', icon: Settings },
  { id: 4, title: 'API Keys', icon: Key },
];

export function OnboardingPage() {
  const navigate = useNavigate();
  useAppStore();
  const [currentStep, setCurrentStep] = useState(1);
  const [formData, setFormData] = useState({
    organizationName: '',
    industry: '',
    website: '',
    selectedConnector: '',
    connectorCredentials: {} as Record<string, string>,
    routingStrategy: 'success_rate',
  });
  const [createdApiKey, setCreatedApiKey] = useState<string | null>(null);

  // Fetch available connectors
  const { data: connectors, isLoading: connectorsLoading } = useQuery({
    queryKey: ['connectors'],
    queryFn: () => api.listConnectors(),
    retry: 2,
  });

  // Create API key mutation
  const createKeyMutation = useMutation({
    mutationFn: api.createApiKey,
    onSuccess: (data) => {
      setCreatedApiKey(data.key);
      toast.success('API key created');
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to create API key');
    },
  });

  const handleNext = async () => {
    if (currentStep === 1) {
      // Validate organization step
      if (!formData.organizationName.trim()) {
        toast.error('Please enter an organization name');
        return;
      }
      // Update organization settings
      try {
        await api.updateOrganizationSettings({
          name: formData.organizationName,
        });
      } catch (error) {
        // Continue even if update fails
        console.error('Failed to update organization:', error);
      }
    }

    if (currentStep === 4) {
      // Complete onboarding
      toast.success('Onboarding complete!');
      navigate('/');
      return;
    }

    if (currentStep < steps.length) {
      setCurrentStep(currentStep + 1);
    }

    // Create API key when reaching step 4
    if (currentStep === 3) {
      createKeyMutation.mutate({
        name: 'Onboarding API Key',
        scopes: ['payments:read', 'payments:write'],
        environment: 'sandbox',
      });
    }
  };

  const handleBack = () => {
    if (currentStep > 1) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleCopyKey = async (key: string) => {
    await navigator.clipboard.writeText(key);
    toast.success('API key copied to clipboard');
  };

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-white border-b border-gray-200">
        <div className="max-w-4xl mx-auto px-6 py-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-primary-600 rounded-lg flex items-center justify-center">
              <Zap className="w-6 h-6 text-white" />
            </div>
            <h1 className="text-xl font-bold text-gray-900">PaymentOrchestra</h1>
          </div>
        </div>
      </header>

      <main className="max-w-4xl mx-auto px-6 py-12">
        {/* Progress Steps */}
        <div className="mb-12">
          <div className="flex items-center justify-between">
            {steps.map((step, index) => (
              <div key={step.id} className="flex items-center">
                <div className="flex items-center gap-3">
                  <div
                    className={clsx(
                      'w-10 h-10 rounded-full flex items-center justify-center transition-colors',
                      currentStep > step.id
                        ? 'bg-success-500 text-white'
                        : currentStep === step.id
                        ? 'bg-primary-600 text-white'
                        : 'bg-gray-200 text-gray-500'
                    )}
                  >
                    {currentStep > step.id ? (
                      <Check className="w-5 h-5" />
                    ) : (
                      <step.icon className="w-5 h-5" />
                    )}
                  </div>
                  <span
                    className={clsx(
                      'text-sm font-medium hidden sm:block',
                      currentStep >= step.id ? 'text-gray-900' : 'text-gray-500'
                    )}
                  >
                    {step.title}
                  </span>
                </div>
                {index < steps.length - 1 && (
                  <div
                    className={clsx(
                      'w-16 sm:w-24 h-0.5 mx-4',
                      currentStep > step.id ? 'bg-success-500' : 'bg-gray-200'
                    )}
                  />
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Step Content */}
        <div className="bg-white rounded-2xl shadow-sm border border-gray-100 p-8">
          {currentStep === 1 && (
            <div className="space-y-6">
              <div>
                <h2 className="text-2xl font-bold text-gray-900">Welcome to PaymentOrchestra!</h2>
                <p className="text-gray-500 mt-2">
                  Let's set up your organization. This information helps us personalize your experience.
                </p>
              </div>

              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Organization Name *
                  </label>
                  <input
                    type="text"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                    placeholder="e.g., Acme Inc"
                    value={formData.organizationName}
                    onChange={(e) =>
                      setFormData({ ...formData, organizationName: e.target.value })
                    }
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Industry *
                  </label>
                  <select
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                    value={formData.industry}
                    onChange={(e) =>
                      setFormData({ ...formData, industry: e.target.value })
                    }
                  >
                    <option value="">Select your industry</option>
                    <option value="ecommerce">E-commerce</option>
                    <option value="saas">SaaS</option>
                    <option value="marketplace">Marketplace</option>
                    <option value="travel">Travel</option>
                    <option value="gaming">Gaming</option>
                    <option value="other">Other</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Website
                  </label>
                  <input
                    type="url"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                    placeholder="https://yourcompany.com"
                    value={formData.website}
                    onChange={(e) =>
                      setFormData({ ...formData, website: e.target.value })
                    }
                  />
                </div>
              </div>
            </div>
          )}

          {currentStep === 2 && (
            <div className="space-y-6">
              <div>
                <h2 className="text-2xl font-bold text-gray-900">Connect Your First Gateway</h2>
                <p className="text-gray-500 mt-2">
                  Select a payment gateway to connect. You can add more later.
                </p>
              </div>

              {connectorsLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="w-6 h-6 text-gray-400 animate-spin" />
                </div>
              ) : connectors && connectors.length > 0 ? (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  {connectors.slice(0, 6).map((connector) => (
                    <button
                      key={connector.id}
                      onClick={() =>
                        setFormData({ ...formData, selectedConnector: connector.id })
                      }
                      className={clsx(
                        'p-4 rounded-xl border-2 text-left transition-all',
                        formData.selectedConnector === connector.id
                          ? 'border-primary-500 bg-primary-50'
                          : 'border-gray-200 hover:border-gray-300'
                      )}
                    >
                      <div className="flex items-center gap-3">
                        <div className="w-10 h-10 bg-gray-100 rounded-lg flex items-center justify-center">
                          <Plug className="w-5 h-5 text-gray-600" />
                        </div>
                        <div>
                          <h3 className="font-medium text-gray-900">{connector.name}</h3>
                          <p className="text-sm text-gray-500">
                            {connector.supported_currencies.length} currencies
                          </p>
                        </div>
                      </div>
                    </button>
                  ))}
                </div>
              ) : (
                <div className="text-center py-12 text-gray-500">
                  No connectors available. You can add one later.
                </div>
              )}

              {formData.selectedConnector && (
                <div className="p-4 bg-gray-50 rounded-xl space-y-4">
                  <h3 className="font-medium text-gray-900">Enter Credentials</h3>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        API Key
                      </label>
                      <input
                        type="password"
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                        placeholder="sk_test_..."
                        value={formData.connectorCredentials.api_key || ''}
                        onChange={(e) =>
                          setFormData({
                            ...formData,
                            connectorCredentials: {
                              ...formData.connectorCredentials,
                              api_key: e.target.value,
                            },
                          })
                        }
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        Secret Key
                      </label>
                      <input
                        type="password"
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                        placeholder="whsec_..."
                        value={formData.connectorCredentials.secret_key || ''}
                        onChange={(e) =>
                          setFormData({
                            ...formData,
                            connectorCredentials: {
                              ...formData.connectorCredentials,
                              secret_key: e.target.value,
                            },
                          })
                        }
                      />
                    </div>
                  </div>
                  <button className="btn-secondary text-sm flex items-center gap-2">
                    <Plug className="w-4 h-4" />
                    Test Connection
                  </button>
                </div>
              )}
            </div>
          )}

          {currentStep === 3 && (
            <div className="space-y-6">
              <div>
                <h2 className="text-2xl font-bold text-gray-900">Set Up Routing</h2>
                <p className="text-gray-500 mt-2">
                  Configure how payments are routed across your gateways.
                </p>
              </div>

              <div className="space-y-4">
                <label className="block text-sm font-medium text-gray-700">
                  Routing Strategy
                </label>
                {[
                  { id: 'success_rate', label: 'Success Rate (Recommended)', description: 'Route to gateways with highest success rates' },
                  { id: 'priority', label: 'Priority', description: 'Route based on priority order' },
                  { id: 'cost', label: 'Cost Optimization', description: 'Route to lowest-cost gateways' },
                ].map((strategy) => (
                  <label
                    key={strategy.id}
                    className={clsx(
                      'flex items-start gap-4 p-4 rounded-xl border-2 cursor-pointer transition-all',
                      formData.routingStrategy === strategy.id
                        ? 'border-primary-500 bg-primary-50'
                        : 'border-gray-200 hover:border-gray-300'
                    )}
                  >
                    <input
                      type="radio"
                      name="routing"
                      value={strategy.id}
                      checked={formData.routingStrategy === strategy.id}
                      onChange={(e) =>
                        setFormData({ ...formData, routingStrategy: e.target.value })
                      }
                      className="mt-1"
                    />
                    <div>
                      <h3 className="font-medium text-gray-900">{strategy.label}</h3>
                      <p className="text-sm text-gray-500">{strategy.description}</p>
                    </div>
                  </label>
                ))}
              </div>
            </div>
          )}

          {currentStep === 4 && (
            <div className="space-y-6">
              <div>
                <h2 className="text-2xl font-bold text-gray-900">Generate API Keys</h2>
                <p className="text-gray-500 mt-2">
                  Create your first API key to start integrating with the platform.
                </p>
              </div>

              {createKeyMutation.isPending ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="w-6 h-6 text-gray-400 animate-spin" />
                </div>
              ) : createdApiKey ? (
                <div className="p-6 bg-success-50 border border-success-200 rounded-xl">
                  <div className="flex items-center gap-4 mb-4">
                    <div className="w-12 h-12 bg-success-100 rounded-full flex items-center justify-center">
                      <CheckCircle className="w-6 h-6 text-success-600" />
                    </div>
                    <div>
                      <h3 className="font-medium text-success-900">API Key Created!</h3>
                      <p className="text-sm text-success-700">
                        Copy this key now - you won't be able to see it again.
                      </p>
                    </div>
                  </div>

                  <div className="space-y-4">
                    <div>
                      <label className="block text-sm font-medium text-success-800 mb-1">
                        Your API Key
                      </label>
                      <div className="flex gap-2">
                        <input
                          type="text"
                          className="flex-1 px-3 py-2 bg-white border border-success-300 rounded-lg font-mono text-sm"
                          value={createdApiKey}
                          readOnly
                        />
                        <button
                          onClick={() => handleCopyKey(createdApiKey)}
                          className="btn-secondary flex items-center gap-2"
                        >
                          <Copy className="w-4 h-4" />
                          Copy
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="text-center py-12">
                  <p className="text-gray-500">Click "Complete Setup" to generate your first API key</p>
                </div>
              )}
            </div>
          )}

          {/* Navigation Buttons */}
          <div className="flex justify-between mt-8 pt-6 border-t border-gray-100">
            <button
              onClick={handleBack}
              disabled={currentStep === 1}
              className={clsx(
                'btn-secondary flex items-center gap-2',
                currentStep === 1 && 'opacity-50 cursor-not-allowed'
              )}
            >
              <ArrowLeft className="w-4 h-4" />
              Back
            </button>
            <button
              onClick={handleNext}
              disabled={createKeyMutation.isPending}
              className="btn-primary flex items-center gap-2"
            >
              {currentStep === steps.length ? 'Complete Setup' : 'Continue'}
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      </main>
    </div>
  );
}
