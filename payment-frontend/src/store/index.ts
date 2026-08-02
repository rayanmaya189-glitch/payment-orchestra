import { create } from 'zustand';
import type { User, Organization } from '@/types';

interface AppState {
  // Auth
  user: User | null;
  organization: Organization | null;
  isAuthenticated: boolean;
  
  // UI
  sidebarOpen: boolean;
  theme: 'light' | 'dark';
  
  // Actions
  setUser: (user: User | null) => void;
  setOrganization: (org: Organization | null) => void;
  login: (user: User, org: Organization, token: string) => void;
  logout: () => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setTheme: (theme: 'light' | 'dark') => void;
}

export const useAppStore = create<AppState>((set) => ({
  // Initial state
  user: null,
  organization: null,
  isAuthenticated: !!localStorage.getItem('auth_token'),
  sidebarOpen: true,
  theme: (localStorage.getItem('theme') as 'light' | 'dark') || 'light',

  // Actions
  setUser: (user) => set({ user }),
  
  setOrganization: (organization) => set({ organization }),
  
  login: (user, organization, token) => {
    localStorage.setItem('auth_token', token);
    set({ user, organization, isAuthenticated: true });
  },
  
  logout: () => {
    localStorage.removeItem('auth_token');
    set({ user: null, organization: null, isAuthenticated: false });
  },
  
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  
  setTheme: (theme) => {
    localStorage.setItem('theme', theme);
    set({ theme });
  },
}));
