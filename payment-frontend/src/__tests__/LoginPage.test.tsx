import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter } from 'react-router-dom';
import { LoginPage } from '@/pages/LoginPage';

vi.mock('@/services/api', () => ({
  api: {
    login: vi.fn(),
    getMe: vi.fn(),
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

describe('LoginPage', () => {
  it('renders login form', () => {
    renderWithProviders(<LoginPage />);
    expect(screen.getByText(/sign in/i)).toBeInTheDocument();
  });

  it('has email and password inputs', () => {
    renderWithProviders(<LoginPage />);
    expect(screen.getByLabelText(/email/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument();
  });

  it('has a submit button', () => {
    renderWithProviders(<LoginPage />);
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument();
  });
});
