import { Routes, Route, Navigate } from 'react-router-dom';
import { useAppStore } from '@/store';
import { Layout } from '@/components/Layout';
import { DashboardPage } from '@/pages/DashboardPage';
import { PaymentsPage } from '@/pages/PaymentsPage';
import { RoutingPage } from '@/pages/RoutingPage';
import { ConnectorsPage } from '@/pages/ConnectorsPage';
import { ApiKeysPage } from '@/pages/ApiKeysPage';
import { AnalyticsPage } from '@/pages/AnalyticsPage';
import { SettingsPage } from '@/pages/SettingsPage';
import { LoginPage } from '@/pages/LoginPage';
import { OnboardingPage } from '@/pages/OnboardingPage';
import { DeveloperPortalPage } from '@/pages/DeveloperPortalPage';
import { WebhooksPage } from '@/pages/WebhooksPage';
import { ReconciliationPage } from '@/pages/ReconciliationPage';
import { AuditLogsPage } from '@/pages/AuditLogsPage';
import { RateLimitsPage } from '@/pages/RateLimitsPage';
import { AiAssistantPage } from '@/pages/AiAssistantPage';
import { SsoPage } from '@/pages/SsoPage';
import { BrandingPage } from '@/pages/BrandingPage';
import { ConnectorMarketplacePage } from '@/pages/ConnectorMarketplacePage';
import { StatusPage } from '@/pages/StatusPage';

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated } = useAppStore();
  
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }
  
  return <>{children}</>;
}

function PublicRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated } = useAppStore();
  
  if (isAuthenticated) {
    return <Navigate to="/" replace />;
  }
  
  return <>{children}</>;
}

export default function App() {
  return (
    <Routes>
      {/* Public Routes */}
      <Route
        path="/login"
        element={
          <PublicRoute>
            <LoginPage />
          </PublicRoute>
        }
      />
      
      {/* Onboarding */}
      <Route
        path="/onboarding"
        element={
          <ProtectedRoute>
            <OnboardingPage />
          </ProtectedRoute>
        }
      />

      {/* Protected Routes */}
      <Route
        path="/"
        element={
          <ProtectedRoute>
            <Layout />
          </ProtectedRoute>
        }
      >
        <Route index element={<DashboardPage />} />
        <Route path="payments" element={<PaymentsPage />} />
        <Route path="routing" element={<RoutingPage />} />
        <Route path="connectors" element={<ConnectorsPage />} />
        <Route path="marketplace" element={<ConnectorMarketplacePage />} />
        <Route path="api-keys" element={<ApiKeysPage />} />
        <Route path="analytics" element={<AnalyticsPage />} />
        <Route path="reconciliation" element={<ReconciliationPage />} />
        <Route path="audit-logs" element={<AuditLogsPage />} />
        <Route path="rate-limits" element={<RateLimitsPage />} />
        <Route path="assistant" element={<AiAssistantPage />} />
        <Route path="sso" element={<SsoPage />} />
        <Route path="branding" element={<BrandingPage />} />
        <Route path="status" element={<StatusPage />} />
        <Route path="developer" element={<DeveloperPortalPage />} />
        <Route path="webhooks" element={<WebhooksPage />} />
        <Route path="settings" element={<SettingsPage />} />
      </Route>

      {/* Catch all */}
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
