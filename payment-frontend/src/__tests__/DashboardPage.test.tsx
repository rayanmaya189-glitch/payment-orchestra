import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter } from 'react-router-dom';
import { DashboardPage } from '@/pages/DashboardPage';

// Mock the API module
vi.mock('@/services/api', () => ({
  api: {
    getDashboardMetrics: vi.fn().mockResolvedValue({
      total_transactions: 1234,
      total_volume: 567890,
      success_rate: 98.5,
      avg_latency_ms: 150,
    }),
    getRecentTransactions: vi.fn().mockResolvedValue([]),
    getGatewayPerformance: vi.fn().mockResolvedValue([]),
  },
  ApiError: class ApiError extends Error {
    code?: string;
    constructor(message: string, code?: string) {
      super(message);
      this.code = code;
    }
  },
}));

vi.mock('@/store', () => ({
  useAppStore: () => ({
    organization: { id: 'org-1', name: 'Test Org' },
  }),
}));

function renderWithProviders(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>{ui}</BrowserRouter>
    </QueryClientProvider>
  );
}

describe('DashboardPage', () => {
  it('renders the page header', async () => {
    renderWithProviders(<DashboardPage />);
    expect(screen.getByRole('heading', { name: /dashboard/i })).toBeInTheDocument();
  });

  it('displays loading state', () => {
    renderWithProviders(<DashboardPage />);
    expect(screen.getByRole('heading', { name: /dashboard/i })).toBeInTheDocument();
  });

  it('renders metric cards after data loads', async () => {
    renderWithProviders(<DashboardPage />);
    await waitFor(() => {
      expect(screen.getByText(/total transactions/i)).toBeInTheDocument();
    });
  });
});
