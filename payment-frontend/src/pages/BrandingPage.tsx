import { useState } from 'react';
import { useBranding } from '@/components/BrandingProvider';
import { Card } from '@/components/ui/Card';
import { Palette, Image, Save, RotateCcw } from 'lucide-react';
import toast from 'react-hot-toast';

export function BrandingPage() {
  const { branding, updateBranding } = useBranding();
  const [primaryColor, setPrimaryColor] = useState(branding.primaryColor);
  const [secondaryColor, setSecondaryColor] = useState(branding.secondaryColor);
  const [companyName, setCompanyName] = useState(branding.companyName);
  const [logoUrl, setLogoUrl] = useState(branding.logoUrl);
  const [faviconUrl, setFaviconUrl] = useState(branding.faviconUrl);

  const handleSave = () => {
    updateBranding({
      primaryColor,
      secondaryColor,
      companyName,
      logoUrl,
      faviconUrl,
    });
    toast.success('Branding settings saved');
  };

  const handleReset = () => {
    setPrimaryColor('#0ea5e9');
    setSecondaryColor('#64748b');
    setCompanyName('PaymentOrchestra');
    setLogoUrl('');
    setFaviconUrl('/favicon.ico');
    updateBranding({
      primaryColor: '#0ea5e9',
      secondaryColor: '#64748b',
      companyName: 'PaymentOrchestra',
      logoUrl: '',
      faviconUrl: '/favicon.ico',
    });
    toast.success('Branding settings reset');
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Custom Branding</h1>
          <p className="text-gray-500">Customize the look and feel of your dashboard</p>
        </div>
        <div className="flex items-center gap-3">
          <button onClick={handleReset} className="btn-secondary flex items-center gap-2">
            <RotateCcw className="w-4 h-4" />
            Reset
          </button>
          <button onClick={handleSave} className="btn-primary flex items-center gap-2">
            <Save className="w-4 h-4" />
            Save Changes
          </button>
        </div>
      </div>

      {/* Preview */}
      <Card>
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Preview</h2>
        <div className="p-6 bg-gray-50 rounded-lg">
          <div className="flex items-center gap-3 mb-4">
            {logoUrl ? (
              <img src={logoUrl} alt="Logo" className="h-10" />
            ) : (
              <div
                className="w-10 h-10 rounded-lg flex items-center justify-center text-white font-bold"
                style={{ backgroundColor: primaryColor }}
              >
                {companyName.charAt(0)}
              </div>
            )}
            <span className="text-xl font-bold" style={{ color: primaryColor }}>
              {companyName}
            </span>
          </div>
          <div className="flex gap-2">
            <button
              className="px-4 py-2 rounded-lg text-white font-medium"
              style={{ backgroundColor: primaryColor }}
            >
              Primary Button
            </button>
            <button
              className="px-4 py-2 rounded-lg font-medium border"
              style={{ borderColor: secondaryColor, color: secondaryColor }}
            >
              Secondary Button
            </button>
          </div>
        </div>
      </Card>

      {/* Branding Settings */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Colors */}
        <Card>
          <div className="flex items-center gap-2 mb-4">
            <Palette className="w-5 h-5 text-gray-600" />
            <h3 className="font-medium text-gray-900">Colors</h3>
          </div>
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Primary Color
              </label>
              <div className="flex items-center gap-3">
                <input
                  type="color"
                  value={primaryColor}
                  onChange={(e) => setPrimaryColor(e.target.value)}
                  className="w-10 h-10 rounded border border-gray-300 cursor-pointer"
                />
                <input
                  type="text"
                  value={primaryColor}
                  onChange={(e) => setPrimaryColor(e.target.value)}
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 font-mono"
                />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Secondary Color
              </label>
              <div className="flex items-center gap-3">
                <input
                  type="color"
                  value={secondaryColor}
                  onChange={(e) => setSecondaryColor(e.target.value)}
                  className="w-10 h-10 rounded border border-gray-300 cursor-pointer"
                />
                <input
                  type="text"
                  value={secondaryColor}
                  onChange={(e) => setSecondaryColor(e.target.value)}
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 font-mono"
                />
              </div>
            </div>
          </div>
        </Card>

        {/* Logo & Brand */}
        <Card>
          <div className="flex items-center gap-2 mb-4">
            <Image className="w-5 h-5 text-gray-600" />
            <h3 className="font-medium text-gray-900">Logo & Brand</h3>
          </div>
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Company Name
              </label>
              <input
                type="text"
                value={companyName}
                onChange={(e) => setCompanyName(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Logo URL
              </label>
              <input
                type="url"
                value={logoUrl}
                onChange={(e) => setLogoUrl(e.target.value)}
                placeholder="https://example.com/logo.png"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Favicon URL
              </label>
              <input
                type="url"
                value={faviconUrl}
                onChange={(e) => setFaviconUrl(e.target.value)}
                placeholder="https://example.com/favicon.ico"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>
          </div>
        </Card>
      </div>

      {/* Color Presets */}
      <Card>
        <h3 className="font-medium text-gray-900 mb-4">Color Presets</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          {[
            { name: 'Blue', primary: '#0ea5e9', secondary: '#64748b' },
            { name: 'Indigo', primary: '#6366f1', secondary: '#64748b' },
            { name: 'Purple', primary: '#8b5cf6', secondary: '#64748b' },
            { name: 'Emerald', primary: '#10b981', secondary: '#64748b' },
            { name: 'Orange', primary: '#f97316', secondary: '#64748b' },
            { name: 'Rose', primary: '#f43f5e', secondary: '#64748b' },
            { name: 'Teal', primary: '#14b8a6', secondary: '#64748b' },
            { name: 'Slate', primary: '#475569', secondary: '#64748b' },
          ].map((preset) => (
            <button
              key={preset.name}
              onClick={() => {
                setPrimaryColor(preset.primary);
                setSecondaryColor(preset.secondary);
              }}
              className="p-3 border border-gray-200 rounded-lg hover:border-primary-300 transition-colors text-left"
            >
              <div className="flex items-center gap-2 mb-2">
                <div
                  className="w-4 h-4 rounded"
                  style={{ backgroundColor: preset.primary }}
                />
                <div
                  className="w-4 h-4 rounded"
                  style={{ backgroundColor: preset.secondary }}
                />
              </div>
              <span className="text-sm font-medium text-gray-700">{preset.name}</span>
            </button>
          ))}
        </div>
      </Card>
    </div>
  );
}
