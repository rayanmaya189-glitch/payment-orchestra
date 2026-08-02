import { createContext, useContext, useEffect, useState, ReactNode } from 'react';

interface BrandingConfig {
  primaryColor: string;
  secondaryColor: string;
  logoUrl: string;
  companyName: string;
  faviconUrl: string;
}

interface BrandingContextType {
  branding: BrandingConfig;
  updateBranding: (config: Partial<BrandingConfig>) => void;
}

const defaultBranding: BrandingConfig = {
  primaryColor: '#0ea5e9',
  secondaryColor: '#64748b',
  logoUrl: '',
  companyName: 'PaymentOrchestra',
  faviconUrl: '/favicon.ico',
};

const BrandingContext = createContext<BrandingContextType>({
  branding: defaultBranding,
  updateBranding: () => {},
});

export function useBranding() {
  return useContext(BrandingContext);
}

interface BrandingProviderProps {
  children: ReactNode;
  initialBranding?: Partial<BrandingConfig>;
}

export function BrandingProvider({ children, initialBranding }: BrandingProviderProps) {
  const [branding, setBranding] = useState<BrandingConfig>(() => {
    const saved = localStorage.getItem('branding_config');
    if (saved) {
      return { ...defaultBranding, ...JSON.parse(saved) };
    }
    return { ...defaultBranding, ...initialBranding };
  });

  useEffect(() => {
    // Apply CSS variables for custom colors
    document.documentElement.style.setProperty('--color-primary', branding.primaryColor);
    document.documentElement.style.setProperty('--color-secondary', branding.secondaryColor);
    
    // Update favicon
    if (branding.faviconUrl) {
      const link = document.querySelector("link[rel~='icon']") as HTMLLinkElement;
      if (link) {
        link.href = branding.faviconUrl;
      }
    }

    // Update document title
    if (branding.companyName) {
      document.title = `${branding.companyName} - Dashboard`;
    }

    // Save to localStorage
    localStorage.setItem('branding_config', JSON.stringify(branding));
  }, [branding]);

  const updateBranding = (config: Partial<BrandingConfig>) => {
    setBranding((prev) => ({ ...prev, ...config }));
  };

  return (
    <BrandingContext.Provider value={{ branding, updateBranding }}>
      {children}
    </BrandingContext.Provider>
  );
}
