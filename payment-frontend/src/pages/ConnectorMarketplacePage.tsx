import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { 
  Search, 
  Filter, 
  Grid, 
  List, 
  Star, 
  Download,
  ExternalLink,
  Zap,
  Shield,
  CreditCard,
  Wallet,
  Building2,
  Smartphone,
  AlertCircle
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api, ApiError } from '@/services/api';
import { clsx } from 'clsx';
import type { Connector } from '@/types';

const categoryConfig = {
  card_processing: {
    label: 'Card Processing',
    icon: CreditCard,
    color: 'bg-blue-500',
    textColor: 'text-blue-600',
    bgLight: 'bg-blue-50',
    description: 'Credit/debit card processing and tokenization'
  },
  wallet: {
    label: 'Digital Wallets',
    icon: Wallet,
    color: 'bg-purple-500',
    textColor: 'text-purple-600',
    bgLight: 'bg-purple-50',
    description: 'Apple Pay, Google Pay, and other digital wallets'
  },
  bank_transfer: {
    label: 'Bank Transfers',
    icon: Building2,
    color: 'bg-green-500',
    textColor: 'text-green-600',
    bgLight: 'bg-green-50',
    description: 'ACH, SEPA, and direct bank transfers'
  },
  bnpl: {
    label: 'Buy Now Pay Later',
    icon: Smartphone,
    color: 'bg-orange-500',
    textColor: 'text-orange-600',
    bgLight: 'bg-orange-50',
    description: 'Installment and deferred payment options'
  },
  alternative: {
    label: 'Alternative Methods',
    icon: Zap,
    color: 'bg-yellow-500',
    textColor: 'text-yellow-600',
    bgLight: 'bg-yellow-50',
    description: 'UPI, PIX, and local payment methods'
  }
};

const regionConfig = {
  global: { label: 'Global', color: 'bg-gray-100 text-gray-800' },
  middle_east: { label: 'Middle East', color: 'bg-emerald-100 text-emerald-800' },
  india: { label: 'India', color: 'bg-orange-100 text-orange-800' },
  europe: { label: 'Europe', color: 'bg-blue-100 text-blue-800' },
  asia_pacific: { label: 'Asia Pacific', color: 'bg-pink-100 text-pink-800' },
  americas: { label: 'Americas', color: 'bg-indigo-100 text-indigo-800' }
};

const pricingConfig = {
  free: { label: 'Free', color: 'bg-green-100 text-green-800' },
  subscription: { label: 'Subscription', color: 'bg-blue-100 text-blue-800' },
  transaction_based: { label: 'Per Transaction', color: 'bg-purple-100 text-purple-800' },
  hybrid: { label: 'Hybrid', color: 'bg-orange-100 text-orange-800' }
};

interface ConnectorWithMeta extends Connector {
  rating: number;
  installs: number;
  last_updated: string;
  region: string[];
  pricing_model: string;
  monthly_price?: number;
  transaction_fee?: number;
  documentation_url?: string;
  support_level: 'community' | 'standard' | 'premium';
}

export function ConnectorMarketplacePage() {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [selectedRegion, setSelectedRegion] = useState<string>('all');
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [sortBy, setSortBy] = useState<'popular' | 'rating' | 'newest' | 'price'>('popular');
  const [showFilters, setShowFilters] = useState(false);

  // Fetch available connectors
  const {
    isLoading,
    error,
  } = useQuery({
    queryKey: ['connectors-marketplace'],
    queryFn: () => api.listConnectors(),
    retry: 2,
  });

  // Mock data for demonstration - in production this would come from API
  const marketplaceConnectors: ConnectorWithMeta[] = [
    {
      id: 'stripe',
      name: 'Stripe',
      description: 'The world\'s most popular payment platform. Accept payments from anyone, anywhere.',
      category: 'card_processing',
      supported_currencies: ['USD', 'EUR', 'GBP', 'AED', 'INR', 'SGD', 'MYR'],
      supported_countries: ['US', 'EU', 'UK', 'AE', 'IN', 'SG', 'MY'],
      features: ['3D Secure', 'Tokenization', 'Subscriptions', 'Connect', 'Radar'],
      requires_credentials: [],
      rating: 4.9,
      installs: 12500,
      last_updated: '2025-01-15',
      region: ['global'],
      pricing_model: 'transaction_based',
      transaction_fee: 2.9,
      documentation_url: 'https://stripe.com/docs',
      support_level: 'premium'
    },
    {
      id: 'adyen',
      name: 'Adyen',
      description: 'Enterprise-grade payment platform with global reach and advanced risk management.',
      category: 'card_processing',
      supported_currencies: ['USD', 'EUR', 'GBP', 'AED', 'INR', 'SGD', 'JPY'],
      supported_countries: ['US', 'EU', 'UK', 'AE', 'IN', 'SG', 'JP'],
      features: ['3D Secure 2.0', 'Tokenization', 'Risk Management', 'Settlement', 'Analytics'],
      requires_credentials: [],
      rating: 4.8,
      installs: 8200,
      last_updated: '2025-01-10',
      region: ['global'],
      pricing_model: 'transaction_based',
      transaction_fee: 2.5,
      documentation_url: 'https://docs.adyen.com',
      support_level: 'premium'
    },
    {
      id: 'checkout_com',
      name: 'Checkout.com',
      description: 'High-performance payment platform built for enterprise scale.',
      category: 'card_processing',
      supported_currencies: ['USD', 'EUR', 'GBP', 'AED'],
      supported_countries: ['US', 'EU', 'UK', 'AE'],
      features: ['Tokenization', 'Risk Engine', '3D Secure', 'Subscriptions'],
      requires_credentials: [],
      rating: 4.7,
      installs: 5600,
      last_updated: '2025-01-08',
      region: ['global', 'middle_east'],
      pricing_model: 'transaction_based',
      transaction_fee: 2.7,
      documentation_url: 'https://docs.checkout.com',
      support_level: 'premium'
    },
    {
      id: 'razorpay',
      name: 'Razorpay',
      description: 'India\'s leading payment gateway with comprehensive payment solutions.',
      category: 'card_processing',
      supported_currencies: ['INR'],
      supported_countries: ['IN'],
      features: ['UPI', 'NetBanking', 'Wallets', 'EMI', 'Subscriptions'],
      requires_credentials: [],
      rating: 4.6,
      installs: 9800,
      last_updated: '2025-01-12',
      region: ['india'],
      pricing_model: 'transaction_based',
      transaction_fee: 2.0,
      documentation_url: 'https://razorpay.com/docs',
      support_level: 'standard'
    },
    {
      id: 'paytabs',
      name: 'PayTabs',
      description: 'Middle East focused payment gateway with local payment method support.',
      category: 'card_processing',
      supported_currencies: ['AED', 'SAR', 'QAR', 'BHD', 'KWD', 'OMR'],
      supported_countries: ['AE', 'SA', 'QA', 'BH', 'KW', 'OM'],
      features: ['3D Secure', 'Tokenization', 'Invoicing', 'Recurring'],
      requires_credentials: [],
      rating: 4.5,
      installs: 3200,
      last_updated: '2025-01-05',
      region: ['middle_east'],
      pricing_model: 'transaction_based',
      transaction_fee: 2.5,
      documentation_url: 'https://paytabs.com/documentation',
      support_level: 'standard'
    },
    {
      id: 'tap_payments',
      name: 'Tap Payments',
      description: 'Simple, secure payment gateway for Middle East and North Africa.',
      category: 'card_processing',
      supported_currencies: ['AED', 'SAR', 'EGP', 'KWD', 'BHD'],
      supported_countries: ['AE', 'SA', 'EG', 'KW', 'BH'],
      features: ['Local Payments', 'Tokenization', 'Webhooks', 'Fraud Protection'],
      requires_credentials: [],
      rating: 4.4,
      installs: 2800,
      last_updated: '2025-01-03',
      region: ['middle_east'],
      pricing_model: 'transaction_based',
      transaction_fee: 2.8,
      documentation_url: 'https://docs.tap.company',
      support_level: 'standard'
    },
    {
      id: 'cashfree',
      name: 'Cashfree',
      description: 'India\'s leading payment aggregator with advanced payout solutions.',
      category: 'card_processing',
      supported_currencies: ['INR'],
      supported_countries: ['IN'],
      features: ['UPI', 'NEFT', 'RTGS', 'Wallets', 'Subscriptions'],
      requires_credentials: [],
      rating: 4.5,
      installs: 4500,
      last_updated: '2025-01-11',
      region: ['india'],
      pricing_model: 'transaction_based',
      transaction_fee: 1.9,
      documentation_url: 'https://docs.cashfree.com',
      support_level: 'standard'
    },
    {
      id: 'ccavenue',
      name: 'CCAvenue',
      description: 'India\'s oldest payment gateway with extensive bank integrations.',
      category: 'card_processing',
      supported_currencies: ['INR'],
      supported_countries: ['IN'],
      features: ['NetBanking', 'EMI', 'Wallets', 'UPI', 'Subscriptions'],
      requires_credentials: [],
      rating: 4.3,
      installs: 6200,
      last_updated: '2025-01-09',
      region: ['india'],
      pricing_model: 'transaction_based',
      transaction_fee: 2.0,
      documentation_url: 'https://www.ccavenue.com',
      support_level: 'standard'
    },
    {
      id: 'mamo_pay',
      name: 'Mamo Pay',
      description: 'UAE\'s digital-first payment platform for modern businesses.',
      category: 'wallet',
      supported_currencies: ['AED'],
      supported_countries: ['AE'],
      features: ['QR Payments', 'P2P Transfers', 'Merchant Payments'],
      requires_credentials: [],
      rating: 4.6,
      installs: 1800,
      last_updated: '2025-01-07',
      region: ['middle_east'],
      pricing_model: 'transaction_based',
      transaction_fee: 2.0,
      documentation_url: 'https://developers.mamopay.com',
      support_level: 'standard'
    },
    {
      id: 'aani',
      name: 'AANI',
      description: 'UAE\'s national payment system for instant bank transfers.',
      category: 'bank_transfer',
      supported_currencies: ['AED'],
      supported_countries: ['AE'],
      features: ['Instant Transfers', 'QR Payments', 'Account Verification'],
      requires_credentials: [],
      rating: 4.7,
      installs: 1200,
      last_updated: '2025-01-06',
      region: ['middle_east'],
      pricing_model: 'transaction_based',
      transaction_fee: 1.5,
      documentation_url: 'https://aani.ae/developers',
      support_level: 'premium'
    },
    {
      id: 'tabby',
      name: 'Tabby',
      description: 'Buy Now Pay Later for Middle East consumers.',
      category: 'bnpl',
      supported_currencies: ['AED', 'SAR'],
      supported_countries: ['AE', 'SA'],
      features: ['Installments', 'Pay in 4', 'Consumer Financing'],
      requires_credentials: [],
      rating: 4.5,
      installs: 2100,
      last_updated: '2025-01-04',
      region: ['middle_east'],
      pricing_model: 'transaction_based',
      transaction_fee: 5.0,
      documentation_url: 'https://docs.tabby.ai',
      support_level: 'standard'
    },
    {
      id: 'tamara',
      name: 'Tamara',
      description: 'Buy Now Pay Later for Middle East and beyond.',
      category: 'bnpl',
      supported_currencies: ['AED', 'SAR', 'KWD', 'BHD'],
      supported_countries: ['AE', 'SA', 'KW', 'BH'],
      features: ['Installments', 'Pay Later', 'Split Payments'],
      requires_credentials: [],
      rating: 4.4,
      installs: 1800,
      last_updated: '2025-01-02',
      region: ['middle_east'],
      pricing_model: 'transaction_based',
      transaction_fee: 4.5,
      documentation_url: 'https://developers.tamara.co',
      support_level: 'standard'
    }
  ];

  // Filter connectors based on search and filters
  const filteredConnectors = marketplaceConnectors.filter(connector => {
    const matchesSearch = 
      connector.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      connector.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      connector.features.some(f => f.toLowerCase().includes(searchQuery.toLowerCase()));
    
    const matchesCategory = selectedCategory === 'all' || connector.category === selectedCategory;
    const matchesRegion = selectedRegion === 'all' || connector.region.includes(selectedRegion);
    
    return matchesSearch && matchesCategory && matchesRegion;
  });

  // Sort connectors
  const sortedConnectors = [...filteredConnectors].sort((a, b) => {
    switch (sortBy) {
      case 'popular':
        return b.installs - a.installs;
      case 'rating':
        return b.rating - a.rating;
      case 'newest':
        return new Date(b.last_updated).getTime() - new Date(a.last_updated).getTime();
      case 'price':
        return (a.transaction_fee || 0) - (b.transaction_fee || 0);
      default:
        return 0;
    }
  });

  const getCategoryStats = () => {
    const stats: Record<string, number> = { all: marketplaceConnectors.length };
    Object.keys(categoryConfig).forEach(cat => {
      stats[cat] = marketplaceConnectors.filter(c => c.category === cat).length;
    });
    return stats;
  };

  const categoryStats = getCategoryStats();

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Connector Marketplace</h1>
          <p className="text-gray-500">Discover and integrate payment providers from around the world</p>
        </div>
        <div className="flex items-center gap-3">
          <div className="relative flex-1 lg:w-80">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
            <input
              type="text"
              placeholder="Search connectors..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10 pr-4 py-2 w-full border border-gray-200 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>
          <button
            onClick={() => setShowFilters(!showFilters)}
            className={clsx(
              'flex items-center gap-2 px-4 py-2 border rounded-lg transition-colors',
              showFilters ? 'bg-primary-50 border-primary-200 text-primary-700' : 'border-gray-200 hover:bg-gray-50'
            )}
          >
            <Filter className="w-4 h-4" />
            Filters
          </button>
          <div className="flex border border-gray-200 rounded-lg overflow-hidden">
            <button
              onClick={() => setViewMode('grid')}
              className={clsx(
                'p-2 transition-colors',
                viewMode === 'grid' ? 'bg-primary-50 text-primary-700' : 'hover:bg-gray-50'
              )}
            >
              <Grid className="w-4 h-4" />
            </button>
            <button
              onClick={() => setViewMode('list')}
              className={clsx(
                'p-2 transition-colors',
                viewMode === 'list' ? 'bg-primary-50 text-primary-700' : 'hover:bg-gray-50'
              )}
            >
              <List className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {/* Filters Panel */}
      {showFilters && (
        <Card className="p-4">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">Category</label>
              <select
                value={selectedCategory}
                onChange={(e) => setSelectedCategory(e.target.value)}
                className="w-full border border-gray-200 rounded-lg px-3 py-2 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              >
                <option value="all">All Categories ({categoryStats.all})</option>
                {Object.entries(categoryConfig).map(([key, config]) => (
                  <option key={key} value={key}>
                    {config.label} ({categoryStats[key] || 0})
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">Region</label>
              <select
                value={selectedRegion}
                onChange={(e) => setSelectedRegion(e.target.value)}
                className="w-full border border-gray-200 rounded-lg px-3 py-2 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              >
                <option value="all">All Regions</option>
                {Object.entries(regionConfig).map(([key, config]) => (
                  <option key={key} value={key}>
                    {config.label}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">Sort By</label>
              <select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as any)}
                className="w-full border border-gray-200 rounded-lg px-3 py-2 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
              >
                <option value="popular">Most Popular</option>
                <option value="rating">Highest Rated</option>
                <option value="newest">Recently Updated</option>
                <option value="price">Lowest Fee</option>
              </select>
            </div>
          </div>
        </Card>
      )}

      {/* Category Stats */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
        {Object.entries(categoryConfig).map(([key, config]) => {
          const Icon = config.icon;
          const count = categoryStats[key] || 0;
          return (
            <button
              key={key}
              onClick={() => setSelectedCategory(selectedCategory === key ? 'all' : key)}
              className={clsx(
                'p-4 rounded-xl border-2 transition-all',
                selectedCategory === key 
                  ? 'border-primary-500 bg-primary-50' 
                  : 'border-gray-200 hover:border-gray-300 bg-white'
              )}
            >
              <div className={clsx('w-10 h-10 rounded-lg flex items-center justify-center mb-3', config.bgLight)}>
                <Icon className={clsx('w-5 h-5', config.textColor)} />
              </div>
              <h3 className="font-medium text-gray-900 text-sm">{config.label}</h3>
              <p className="text-xs text-gray-500 mt-1">{count} connectors</p>
            </button>
          );
        })}
      </div>

      {/* Results Count */}
      <div className="flex items-center justify-between">
        <p className="text-sm text-gray-600">
          Showing {sortedConnectors.length} of {marketplaceConnectors.length} connectors
        </p>
        <div className="flex items-center gap-2 text-sm text-gray-500">
          <Zap className="w-4 h-4" />
          <span>Smart Routing Enabled</span>
        </div>
      </div>

      {/* Connectors Grid/List */}
      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {Array.from({ length: 6 }).map((_, i) => (
            <Card key={i} className="animate-pulse">
              <div className="p-6">
                <div className="flex items-start gap-4 mb-4">
                  <div className="w-12 h-12 bg-gray-200 rounded-xl"></div>
                  <div className="flex-1">
                    <div className="h-4 bg-gray-200 rounded w-24 mb-2"></div>
                    <div className="h-3 bg-gray-200 rounded w-32"></div>
                  </div>
                </div>
                <div className="h-16 bg-gray-200 rounded-lg mb-4"></div>
                <div className="flex gap-2">
                  <div className="h-8 bg-gray-200 rounded-lg flex-1"></div>
                  <div className="h-8 bg-gray-200 rounded-lg flex-1"></div>
                </div>
              </div>
            </Card>
          ))}
        </div>
      ) : error ? (
        <Card className="p-8 text-center">
          <div className="text-gray-400 mb-4">
            <AlertCircle className="w-12 h-12 mx-auto" />
          </div>
          <h3 className="text-lg font-medium text-gray-900 mb-2">Failed to load connectors</h3>
          <p className="text-gray-500 mb-4">
            {error instanceof ApiError ? error.message : 'An error occurred while fetching connectors'}
          </p>
          <button className="btn-primary">Retry</button>
        </Card>
      ) : viewMode === 'grid' ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {sortedConnectors.map((connector) => {
            const category = categoryConfig[connector.category];
            const CategoryIcon = category.icon;
            
            return (
              <Card key={connector.id} className="hover:shadow-lg transition-shadow group">
                <div className="p-6">
                  {/* Header */}
                  <div className="flex items-start gap-4 mb-4">
                    <div className={clsx(
                      'w-12 h-12 rounded-xl flex items-center justify-center',
                      category.bgLight
                    )}>
                      <CategoryIcon className={clsx('w-6 h-6', category.textColor)} />
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="font-semibold text-gray-900 group-hover:text-primary-600 transition-colors">
                        {connector.name}
                      </h3>
                      <div className="flex items-center gap-2 mt-1">
                        <div className="flex items-center">
                          <Star className="w-3 h-3 text-yellow-400 fill-current" />
                          <span className="text-xs text-gray-600 ml-1">{connector.rating}</span>
                        </div>
                        <span className="text-gray-300">•</span>
                        <div className="flex items-center">
                          <Download className="w-3 h-3 text-gray-400" />
                          <span className="text-xs text-gray-600 ml-1">{connector.installs.toLocaleString()}</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Description */}
                  <p className="text-sm text-gray-600 mb-4 line-clamp-2">
                    {connector.description}
                  </p>

                  {/* Region & Pricing */}
                  <div className="flex flex-wrap gap-2 mb-4">
                    {connector.region.map((r) => (
                      <span
                        key={r}
                        className={clsx(
                          'px-2 py-1 text-xs font-medium rounded-full',
                          regionConfig[r as keyof typeof regionConfig]?.color || 'bg-gray-100 text-gray-800'
                        )}
                      >
                        {regionConfig[r as keyof typeof regionConfig]?.label || r}
                      </span>
                    ))}
                    <span className={clsx(
                      'px-2 py-1 text-xs font-medium rounded-full',
                      pricingConfig[connector.pricing_model as keyof typeof pricingConfig]?.color || 'bg-gray-100 text-gray-800'
                    )}>
                      {connector.pricing_model === 'transaction_based' 
                        ? `${connector.transaction_fee}% per txn`
                        : pricingConfig[connector.pricing_model as keyof typeof pricingConfig]?.label
                      }
                    </span>
                  </div>

                  {/* Features */}
                  <div className="flex flex-wrap gap-1 mb-4">
                    {connector.features.slice(0, 3).map((feature) => (
                      <span
                        key={feature}
                        className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded"
                      >
                        {feature}
                      </span>
                    ))}
                    {connector.features.length > 3 && (
                      <span className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded">
                        +{connector.features.length - 3}
                      </span>
                    )}
                  </div>

                  {/* Support Level */}
                  <div className="flex items-center gap-2 mb-4">
                    <Shield className="w-4 h-4 text-gray-400" />
                    <span className="text-xs text-gray-500 capitalize">
                      {connector.support_level} Support
                    </span>
                  </div>

                  {/* Actions */}
                  <div className="flex gap-2">
                    <button className="flex-1 btn-primary text-sm flex items-center justify-center gap-1">
                      <Download className="w-4 h-4" />
                      Install
                    </button>
                    <button className="flex-1 btn-secondary text-sm flex items-center justify-center gap-1">
                      <ExternalLink className="w-4 h-4" />
                      Docs
                    </button>
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      ) : (
        /* List View */
        <div className="space-y-4">
          {sortedConnectors.map((connector) => {
            const category = categoryConfig[connector.category];
            const CategoryIcon = category.icon;
            
            return (
              <Card key={connector.id} className="hover:shadow-md transition-shadow">
                <div className="p-6">
                  <div className="flex items-center gap-6">
                    {/* Icon */}
                    <div className={clsx(
                      'w-16 h-16 rounded-xl flex items-center justify-center flex-shrink-0',
                      category.bgLight
                    )}>
                      <CategoryIcon className={clsx('w-8 h-8', category.textColor)} />
                    </div>

                    {/* Content */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-start justify-between">
                        <div>
                          <h3 className="font-semibold text-gray-900 text-lg">{connector.name}</h3>
                          <p className="text-sm text-gray-500 mt-1">{connector.description}</p>
                        </div>
                        <div className="flex items-center gap-4 flex-shrink-0">
                          <div className="text-right">
                            <div className="flex items-center">
                              <Star className="w-4 h-4 text-yellow-400 fill-current" />
                              <span className="font-medium text-gray-900 ml-1">{connector.rating}</span>
                            </div>
                            <p className="text-xs text-gray-500">{connector.installs.toLocaleString()} installs</p>
                          </div>
                          <div className="flex flex-col gap-1">
                            {connector.region.map((r) => (
                              <span
                                key={r}
                                className={clsx(
                                  'px-2 py-0.5 text-xs font-medium rounded-full text-center',
                                  regionConfig[r as keyof typeof regionConfig]?.color || 'bg-gray-100 text-gray-800'
                                )}
                              >
                                {regionConfig[r as keyof typeof regionConfig]?.label || r}
                              </span>
                            ))}
                          </div>
                        </div>
                      </div>

                      {/* Features & Pricing */}
                      <div className="flex items-center gap-4 mt-3">
                        <div className="flex flex-wrap gap-1">
                          {connector.features.slice(0, 4).map((feature) => (
                            <span
                              key={feature}
                              className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded"
                            >
                              {feature}
                            </span>
                          ))}
                          {connector.features.length > 4 && (
                            <span className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded">
                              +{connector.features.length - 4}
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-2 text-sm text-gray-500">
                          <span className="font-medium text-gray-900">
                            {connector.pricing_model === 'transaction_based' 
                              ? `${connector.transaction_fee}%`
                              : 'Free'
                            }
                          </span>
                          <span>per transaction</span>
                        </div>
                      </div>
                    </div>

                    {/* Actions */}
                    <div className="flex flex-col gap-2 flex-shrink-0">
                      <button className="btn-primary text-sm flex items-center gap-1">
                        <Download className="w-4 h-4" />
                        Install
                      </button>
                      <button className="btn-secondary text-sm flex items-center gap-1">
                        <ExternalLink className="w-4 h-4" />
                        Docs
                      </button>
                    </div>
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      )}

      {/* Empty State */}
      {!isLoading && !error && sortedConnectors.length === 0 && (
        <Card className="p-8 text-center">
          <Search className="w-12 h-12 text-gray-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900 mb-2">No connectors found</h3>
          <p className="text-gray-500 mb-4">
            Try adjusting your search or filters to find what you're looking for.
          </p>
          <button 
            onClick={() => {
              setSearchQuery('');
              setSelectedCategory('all');
              setSelectedRegion('all');
            }}
            className="btn-secondary"
          >
            Clear Filters
          </button>
        </Card>
      )}

      {/* CTA Section */}
      <Card className="bg-gradient-to-r from-primary-500 to-primary-600 text-white">
        <div className="p-8 text-center">
          <h2 className="text-2xl font-bold mb-2">Need a Custom Integration?</h2>
          <p className="text-primary-100 mb-6 max-w-2xl mx-auto">
            Don't see your payment provider? We can build a custom connector for any payment gateway 
            or banking partner. Our team has integrated with 100+ providers globally.
          </p>
          <div className="flex items-center justify-center gap-4">
            <button className="bg-white text-primary-600 px-6 py-3 rounded-lg font-medium hover:bg-gray-50 transition-colors">
              Request Custom Connector
            </button>
            <button className="border border-white/30 text-white px-6 py-3 rounded-lg font-medium hover:bg-white/10 transition-colors">
              Contact Sales
            </button>
          </div>
        </div>
      </Card>
    </div>
  );
}
