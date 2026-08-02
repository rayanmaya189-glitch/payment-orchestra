import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { 
  Zap, 
  Building2, 
  CreditCard, 
  Settings, 
  Check,
  ArrowRight,
  ArrowLeft,
  Plug,
  Key,
} from 'lucide-react';
import { clsx } from 'clsx';
import toast from 'react-hot-toast';

const steps = [
  { id: 1, title: 'Organization', icon: Building2 },
  { id: 2, title: 'First Connector', icon: Plug },
  { id: 3, title: 'Routing Setup', icon: Settings },
  { id: 4, title: 'API Keys', icon: Key },
];

const connectors = [
  { id: 'stripe', name: 'Stripe', description: 'Accept payments globally' },
  { id: 'checkout_com', name: 'Checkout.com', description: 'Enterprise payment processing' },
  { id: 'adyen', name: 'Adyen', description: 'Global payment platform' },
  { id: 'razorpay', name: 'Razorpay', description: 'India-focused payments' },
  { id: 'network_intl', name: 'Network International', description: 'UAE payment solutions' },
];

export function OnboardingPage() {
  const navigate = useNavigate();
  const [currentStep, setCurrentStep] = useState(1);
  const [formData, setFormData] = useState({
    organizationName: '',
    industry: '',
    website: '',
    selectedConnector: '',
    connectorCredentials: {},
    routingStrategy: 'success_rate',
  });

  const handleNext = () => {
    if (currentStep < steps.length) {
      setCurrentStep(currentStep + 1);
    } else {
      toast.success('Onboarding complete!');
      navigate('/');
    }
  };

  const handleBack = () => {
    if (currentStep > 1) {
      setCurrentStep(currentStep - 1);
    }
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
                  <label className="label">Organization Name *</label>
                  <input
                    type="text"
                    className="input"
                    placeholder="e.g., Acme Inc"
                    value={formData.organizationName}
                    onChange={(e) =>
                      setFormData({ ...formData, organizationName: e.target.value })
                    }
                  />
                </div>
                <div>
                  <label className="label">Industry *</label>
                  <select
                    className="input"
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
                  <label className="label">Website</label>
                  <input
                    type="url"
                    className="input"
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

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {connectors.map((connector) => (
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
                        <CreditCard className="w-5 h-5 text-gray-600" />
                      </div>
                      <div>
                        <h3 className="font-medium text-gray-900">{connector.name}</h3>
                        <p className="text-sm text-gray-500">{connector.description}</p>
                      </div>
                    </div>
                  </button>
                ))}
              </div>

              {formData.selectedConnector && (
                <div className="p-4 bg-gray-50 rounded-xl space-y-4">
                  <h3 className="font-medium text-gray-900">Enter Credentials</h3>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div>
                      <label className="label">API Key</label>
                      <input type="password" className="input" placeholder="sk_test_..." />
                    </div>
                    <div>
                      <label className="label">Secret Key</label>
                      <input type="password" className="input" placeholder="whsec_..." />
                    </div>
                  </div>
                  <button className="btn-secondary text-sm">
                    <Plug className="w-4 h-4 mr-2" />
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
                <label className="label">Routing Strategy</label>
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

              <div className="p-6 bg-gray-50 rounded-xl">
                <div className="flex items-center gap-4 mb-4">
                  <div className="w-12 h-12 bg-success-100 rounded-full flex items-center justify-center">
                    <Check className="w-6 h-6 text-success-600" />
                  </div>
                  <div>
                    <h3 className="font-medium text-gray-900">You're all set!</h3>
                    <p className="text-sm text-gray-500">
                      Your API keys are ready. Use them to integrate with the platform.
                    </p>
                  </div>
                </div>

                <div className="space-y-4">
                  <div>
                    <label className="label">Sandbox API Key</label>
                    <div className="flex gap-2">
                      <input
                        type="text"
                        className="input font-mono"
                        value="pk_sandbox_xxxxxxxxxxxxxxxxxxxxxxxx"
                        readOnly
                      />
                      <button className="btn-secondary">Copy</button>
                    </div>
                  </div>
                  <div>
                    <label className="label">Production API Key</label>
                    <div className="flex gap-2">
                      <input
                        type="text"
                        className="input font-mono"
                        value="pk_live_xxxxxxxxxxxxxxxxxxxxxxxx"
                        readOnly
                      />
                      <button className="btn-secondary">Copy</button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Navigation Buttons */}
          <div className="flex justify-between mt-8 pt-6 border-t border-gray-100">
            <button
              onClick={handleBack}
              disabled={currentStep === 1}
              className={clsx(
                'btn-secondary',
                currentStep === 1 && 'opacity-50 cursor-not-allowed'
              )}
            >
              <ArrowLeft className="w-4 h-4 mr-2" />
              Back
            </button>
            <button onClick={handleNext} className="btn-primary">
              {currentStep === steps.length ? 'Complete Setup' : 'Continue'}
              <ArrowRight className="w-4 h-4 ml-2" />
            </button>
          </div>
        </div>
      </main>
    </div>
  );
}
