import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter } from 'react-router-dom';
import { LoginPage } from '@/pages/LoginPage';

vi.mock('@/services/api', () => ({
  api: {
    login: vi.fn().mockResolvedValue({
      user: { id: '1', email: 'test@test.com', name: 'Test', role: 'admin' },
      organization: { id: 'org-1', name: 'Test Org', tier: 'starter', status: 'active' },
      token: 'test-token',
    }),
    getMe: vi.fn(),
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
    login: vi.fn(),
    organization: null,
  }),
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
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
    expect(screen.getByText(/sign in to your dashboard/i)).toBeInTheDocument();
  });

  it('has email and password inputs', () => {
    renderWithProviders(<LoginPage />);
    expect(screen.getByPlaceholderText('you@company.com')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('••••••••')).toBeInTheDocument();
  });

  it('has a submit button', () => {
    renderWithProviders(<LoginPage />);
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument();
  });
});
