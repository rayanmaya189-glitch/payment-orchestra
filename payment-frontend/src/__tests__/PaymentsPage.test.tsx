import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter } from 'react-router-dom';
import { PaymentsPage } from '@/pages/PaymentsPage';

vi.mock('@/services/api', () => ({
  api: {
    listPaymentIntents: vi.fn().mockResolvedValue({ items: [], total: 0 }),
  },
}));

function renderWithProviders(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>{ui}</BrowserRouter>
    </QueryClientProvider>
  );
}

describe('PaymentsPage', () => {
  it('renders the payments page', async () => {
    renderWithProviders(<PaymentsPage />);
    expect(screen.getByRole('heading', { name: /payments/i })).toBeInTheDocument();
  });

  it('shows payment list container', async () => {
    renderWithProviders(<PaymentsPage />);
    await waitFor(() => {
      expect(screen.getByText(/view and manage all payment transactions/i)).toBeInTheDocument();
    });
  });
});
