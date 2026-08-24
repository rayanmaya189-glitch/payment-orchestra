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
    expect(screen.getByText(/dashboard/i)).toBeInTheDocument();
  });

  it('displays loading state', () => {
    renderWithProviders(<DashboardPage />);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it('renders metric cards after data loads', async () => {
    renderWithProviders(<DashboardPage />);
    await waitFor(() => {
      expect(screen.getByText(/transactions/i)).toBeInTheDocument();
    });
  });
});
