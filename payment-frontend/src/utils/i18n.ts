/**
 * Internationalization (i18n) utility for Payment Orchestra.
 *
 * Supports:
 * - English (en) - Default
 * - Arabic (ar) - RTL support
 *
 * Usage:
 *   import { t, setLocale, getLocale } from '@/utils/i18n';
 *   const label = t('dashboard.title');
 */

export type Locale = 'en' | 'ar';

// ─── Translation Strings ─────────────────────────────────────────────────────

const translations: Record<Locale, Record<string, string>> = {
  en: {
    // Navigation
    'nav.dashboard': 'Dashboard',
    'nav.payments': 'Payments',
    'nav.routing': 'Routing',
    'nav.connectors': 'Connectors',
    'nav.marketplace': 'Marketplace',
    'nav.analytics': 'Analytics',
    'nav.reconciliation': 'Reconciliation',
    'nav.webhooks': 'Webhooks',
    'nav.api_keys': 'API Keys',
    'nav.settings': 'Settings',
    'nav.audit_logs': 'Audit Logs',
    'nav.ai_assistant': 'AI Assistant',
    'nav.developer': 'Developer Portal',
    'nav.sso': 'SSO',
    'nav.branding': 'Branding',

    // Dashboard
    'dashboard.title': 'Dashboard',
    'dashboard.welcome': 'Welcome back! Here\'s what\'s happening with your payments.',
    'dashboard.total_transactions': 'Total Transactions',
    'dashboard.success_rate': 'Success Rate',
    'dashboard.total_volume': 'Total Volume',
    'dashboard.active_gateways': 'Active Gateways',
    'dashboard.transaction_volume': 'Transaction Volume',
    'dashboard.recent_transactions': 'Recent Transactions',
    'dashboard.gateway_performance': 'Gateway Performance',
    'dashboard.view_all': 'View all',
    'dashboard.refresh': 'Refresh',

    // Payments
    'payments.title': 'Payments',
    'payments.subtitle': 'View and manage all payment transactions',
    'payments.search': 'Search by payment ID, order reference, or amount...',
    'payments.export': 'Export',
    'payments.payment_id': 'Payment ID',
    'payments.order': 'Order',
    'payments.amount': 'Amount',
    'payments.status': 'Status',
    'payments.gateway': 'Gateway',
    'payments.risk': 'Risk',
    'payments.time': 'Time',
    'payments.actions': 'Actions',
    'payments.view': 'View',
    'payments.no_payments': 'No payments yet',
    'payments.no_results': 'No payments found',
    'payments.try_adjusting': 'Try adjusting your search or filters',

    // Statuses
    'status.created': 'Created',
    'status.authorizing': 'Authorizing',
    'status.authorized': 'Authorized',
    'status.captured': 'Captured',
    'status.failed': 'Failed',
    'status.refunded': 'Refunded',
    'status.voided': 'Voided',
    'status.partially_refunded': 'Partially Refunded',

    // Routing
    'routing.title': 'Routing Policies',
    'routing.subtitle': 'Configure how payments are routed across your gateways',
    'routing.create_policy': 'Create Policy',
    'routing.total_policies': 'Total Policies',
    'routing.active_policies': 'Active Policies',
    'routing.total_rules': 'Total Rules',
    'routing.activate': 'Activate',
    'routing.deactivate': 'Deactivate',
    'routing.no_policies': 'Create Your First Routing Policy',
    'routing.no_policies_desc': 'Define routing rules to optimize your payment success rates',

    // Connectors
    'connectors.title': 'Connectors',
    'connectors.subtitle': 'Manage your payment gateway connections',
    'connectors.add_connector': 'Add Connector',
    'connectors.test': 'Test',
    'connectors.configure': 'Configure',
    'connectors.success_rate': 'Success Rate',
    'connectors.avg_latency': 'Avg Latency',
    'connectors.available_integrations': 'Available Integrations',
    'connectors.select_provider': 'Select a payment provider to connect.',

    // Common
    'common.loading': 'Loading...',
    'common.error': 'Error',
    'common.retry': 'Retry',
    'common.save': 'Save',
    'common.cancel': 'Cancel',
    'common.delete': 'Delete',
    'common.edit': 'Edit',
    'common.create': 'Create',
    'common.search': 'Search',
    'common.filter': 'Filter',
    'common.all': 'All',
    'common.none': 'None',
    'common.yes': 'Yes',
    'common.no': 'No',
    'common.previous': 'Previous',
    'common.next': 'Next',
    'common.page': 'Page',
    'common.of': 'of',
    'common.showing': 'Showing',
    'common.to': 'to',
    'common.entries': 'entries',

    // Auth
    'auth.login': 'Login',
    'auth.register': 'Register',
    'auth.email': 'Email',
    'auth.password': 'Password',
    'auth.forgot_password': 'Forgot Password?',
    'auth.no_account': 'Don\'t have an account?',
    'auth.has_account': 'Already have an account?',
    'auth.sign_in': 'Sign In',
    'auth.sign_up': 'Sign Up',

    // Time
    'time.just_now': 'Just now',
    'time.minutes_ago': '{n} minutes ago',
    'time.hours_ago': '{n} hours ago',
    'time.days_ago': '{n} days ago',

    // Currency
    'currency.usd': 'USD',
    'currency.aed': 'AED',
    'currency.inr': 'INR',
    'currency.eur': 'EUR',
    'currency.gbp': 'GBP',
  },

  ar: {
    // Navigation
    'nav.dashboard': 'لوحة التحكم',
    'nav.payments': 'المدفوعات',
    'nav.routing': 'التوجيه',
    'nav.connectors': 'الموصلات',
    'nav.marketplace': 'السوق',
    'nav.analytics': 'التحليلات',
    'nav.reconciliation': 'التسوية',
    'nav.webhooks': 'Webhooks',
    'nav.api_keys': 'مفاتيح API',
    'nav.settings': 'الإعدادات',
    'nav.audit_logs': 'سجلات التدقيق',
    'nav.ai_assistant': 'المساعد الذكي',
    'nav.developer': 'بوابة المطورين',
    'nav.sso': 'تسجيل الدخول الموحد',
    'nav.branding': 'العلامة التجارية',

    // Dashboard
    'dashboard.title': 'لوحة التحكم',
    'dashboard.welcome': 'مرحباً بعودتك! إليك ما يحدث مع مدفوعاتك.',
    'dashboard.total_transactions': 'إجمالي المعاملات',
    'dashboard.success_rate': 'معدل النجاح',
    'dashboard.total_volume': 'إجمالي الحجم',
    'dashboard.active_gateways': 'بوابات نشطة',
    'dashboard.transaction_volume': 'حجم المعاملات',
    'dashboard.recent_transactions': 'المعاملات الأخيرة',
    'dashboard.gateway_performance': 'أداء البوابة',
    'dashboard.view_all': 'عرض الكل',
    'dashboard.refresh': 'تحديث',

    // Payments
    'payments.title': 'المدفوعات',
    'payments.subtitle': 'عرض وإدارة جميع معاملات الدفع',
    'payments.search': 'البحث برقم الدفع أو رقم الطلب أو المبلغ...',
    'payments.export': 'تصدير',
    'payments.payment_id': 'رقم الدفع',
    'payments.order': 'الطلب',
    'payments.amount': 'المبلغ',
    'payments.status': 'الحالة',
    'payments.gateway': 'البوابة',
    'payments.risk': 'المخاطر',
    'payments.time': 'الوقت',
    'payments.actions': 'الإجراءات',
    'payments.view': 'عرض',
    'payments.no_payments': 'لا توجد مدفوعات بعد',
    'payments.no_results': 'لم يتم العثور على مدفوعات',
    'payments.try_adjusting': 'حاول تعديل البحث أو الفلاتر',

    // Statuses
    'status.created': 'تم الإنشاء',
    'status.authorizing': 'جارٍ التفويض',
    'status.authorized': 'تم التفويض',
    'status.captured': 'تم الالتقاط',
    'status.failed': 'فشل',
    'status.refunded': 'تم استرداد',
    'status.voided': 'تم الإلغاء',
    'status.partially_refunded': 'استرداد جزئي',

    // Routing
    'routing.title': 'سياسات التوجيه',
    'routing.subtitle': 'تكوين كيفية توجيه المدفوعات عبر بواباتك',
    'routing.create_policy': 'إنشاء سياسة',
    'routing.total_policies': 'إجمالي السياسات',
    'routing.active_policies': 'السياسات النشطة',
    'routing.total_rules': 'إجمالي القواعد',
    'routing.activate': 'تفعيل',
    'routing.deactivate': 'تعطيل',
    'routing.no_policies': 'أنشئ سياسة التوجيه الأولى',
    'routing.no_policies_desc': 'حدد قواعد التوجيه لتحسين معدلات نجاح المدفوعات',

    // Connectors
    'connectors.title': 'الموصلات',
    'connectors.subtitle': 'إدارة اتصالات بوابات الدفع',
    'connectors.add_connector': 'إضافة موصل',
    'connectors.test': 'اختبار',
    'connectors.configure': 'تكوين',
    'connectors.success_rate': 'معدل النجاح',
    'connectors.avg_latency': 'متوسط زمن الاستجابة',
    'connectors.available_integrations': 'التكاملات المتاحة',
    'connectors.select_provider': 'اختر مزود الدفع للاتصال.',

    // Common
    'common.loading': 'جارٍ التحميل...',
    'common.error': 'خطأ',
    'common.retry': 'إعادة المحاولة',
    'common.save': 'حفظ',
    'common.cancel': 'إلغاء',
    'common.delete': 'حذف',
    'common.edit': 'تعديل',
    'common.create': 'إنشاء',
    'common.search': 'بحث',
    'common.filter': 'فلتر',
    'common.all': 'الكل',
    'common.none': 'لا شيء',
    'common.yes': 'نعم',
    'common.no': 'لا',
    'common.previous': 'السابق',
    'common.next': 'التالي',
    'common.page': 'صفحة',
    'common.of': 'من',
    'common.showing': 'عرض',
    'common.to': 'إلى',
    'common.entries': 'entries',

    // Auth
    'auth.login': 'تسجيل الدخول',
    'auth.register': 'التسجيل',
    'auth.email': 'البريد الإلكتروني',
    'auth.password': 'كلمة المرور',
    'auth.forgot_password': 'نسيت كلمة المرور؟',
    'auth.no_account': 'ليس لديك حساب؟',
    'auth.has_account': 'لديك حساب بالفعل؟',
    'auth.sign_in': 'تسجيل الدخول',
    'auth.sign_up': 'التسجيل',

    // Time
    'time.just_now': 'الآن',
    'time.minutes_ago': 'منذ {n} دقائق',
    'time.hours_ago': 'منذ {n} ساعات',
    'time.days_ago': 'منذ {n} أيام',

    // Currency
    'currency.usd': 'دولار',
    'currency.aed': 'درهم',
    'currency.inr': 'روبيه',
    'currency.eur': 'يورو',
    'currency.gbp': 'جنيه',
  },
};

// ─── State ───────────────────────────────────────────────────────────────────

let currentLocale: Locale = 'en';

// ─── Public API ──────────────────────────────────────────────────────────────

/**
 * Get the current locale.
 */
export function getLocale(): Locale {
  return currentLocale;
}

/**
 * Set the current locale.
 */
export function setLocale(locale: Locale): void {
  currentLocale = locale;
  document.documentElement.dir = locale === 'ar' ? 'rtl' : 'ltr';
  document.documentElement.lang = locale;
}

/**
 * Translate a key.
 *
 * @param key - Translation key (e.g., 'dashboard.title')
 * @param params - Optional parameters for interpolation (e.g., { n: 5 })
 * @returns Translated string
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const locale = currentLocale;
  let translation = translations[locale]?.[key] || translations.en[key] || key;

  // Interpolate parameters
  if (params) {
    for (const [param, value] of Object.entries(params)) {
      translation = translation.replace(`{${param}}`, String(value));
    }
  }

  return translation;
}

/**
 * Get text direction for current locale.
 */
export function getDirection(): 'ltr' | 'rtl' {
  return currentLocale === 'ar' ? 'rtl' : 'ltr';
}

/**
 * Get locale display name.
 */
export function getLocaleDisplayName(locale: Locale): string {
  return locale === 'ar' ? 'العربية' : 'English';
}

/**
 * Format currency based on locale.
 */
export function formatCurrency(amount: number, currency: string): string {
  const formatter = new Intl.NumberFormat(currentLocale === 'ar' ? 'ar-AE' : 'en-US', {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
  });
  return formatter.format(amount / 100);
}

/**
 * Format number based on locale.
 */
export function formatNumber(num: number): string {
  const formatter = new Intl.NumberFormat(currentLocale === 'ar' ? 'ar-AE' : 'en-US');
  return formatter.format(num);
}

/**
 * Format percentage based on locale.
 */
export function formatPercentage(num: number): string {
  const formatter = new Intl.NumberFormat(currentLocale === 'ar' ? 'ar-AE' : 'en-US', {
    style: 'percent',
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  });
  return formatter.format(num / 100);
}
