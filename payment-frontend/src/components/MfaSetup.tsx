import { useState, useEffect } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Shield,
  Smartphone,
  Key,
  Copy,
  Check,
  AlertCircle,
  RefreshCw,
  X,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api, ApiError } from '@/services/api';
import { clsx } from 'clsx';

type MfaStep = 'method' | 'setup' | 'verify' | 'backup' | 'complete';

interface MfaSetupProps {
  onComplete?: () => void;
  onCancel?: () => void;
}

export function MfaSetup({ onComplete, onCancel }: MfaSetupProps) {
  const [step, setStep] = useState<MfaStep>('method');
  const [selectedMethod, setSelectedMethod] = useState<'totp' | 'sms' | 'email'>('totp');
  const [secret, setSecret] = useState<string>('');
  const [qrCodeUrl, setQrCodeUrl] = useState<string>('');
  const [verificationCode, setVerificationCode] = useState('');
  const [backupCodes, setBackupCodes] = useState<string[]>([]);
  const [copiedCode, setCopiedCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const queryClient = useQueryClient();

  // Setup MFA mutation
  const setupMutation = useMutation({
    mutationFn: () => api.setupMfa(selectedMethod),
    onSuccess: (data) => {
      setSecret(data.secret);
      setQrCodeUrl(data.qr_code_url);
      setBackupCodes(data.backup_codes);
      setStep('setup');
    },
    onError: (err) => {
      setError(err instanceof ApiError ? err.message : 'Failed to setup MFA');
    },
  });

  // Verify MFA mutation
  const verifyMutation = useMutation({
    mutationFn: (code: string) => api.verifyMfa(code),
    onSuccess: () => {
      setStep('backup');
    },
    onError: (err) => {
      setError(err instanceof ApiError ? err.message : 'Invalid verification code');
    },
  });

  // Enable MFA mutation
  const enableMutation = useMutation({
    mutationFn: () => api.enableMfa(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['user'] });
      setStep('complete');
      onComplete?.();
    },
    onError: (err) => {
      setError(err instanceof ApiError ? err.message : 'Failed to enable MFA');
    },
  });

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedCode(text);
    setTimeout(() => setCopiedCode(null), 2000);
  };

  const handleVerify = () => {
    if (verificationCode.length === 6) {
      setError(null);
      verifyMutation.mutate(verificationCode);
    }
  };

  // Step: Select Method
  if (step === 'method') {
    return (
      <Card className="max-w-md mx-auto">
        <div className="p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-semibold text-gray-900">Enable 2FA</h2>
            <button onClick={onCancel} className="text-gray-400 hover:text-gray-600">
              <X className="w-5 h-5" />
            </button>
          </div>

          <p className="text-sm text-gray-500 mb-6">
            Choose a two-factor authentication method to secure your account.
          </p>

          <div className="space-y-3">
            {[
              { id: 'totp', label: 'Authenticator App', description: 'Use Google Authenticator, Authy, or similar', icon: Smartphone },
              { id: 'sms', label: 'SMS Code', description: 'Receive codes via text message', icon: Smartphone },
              { id: 'email', label: 'Email Code', description: 'Receive codes via email', icon: Key },
            ].map((method) => (
              <button
                key={method.id}
                onClick={() => {
                  setSelectedMethod(method.id as any);
                  setupMutation.mutate();
                }}
                disabled={setupMutation.isPending}
                className={clsx(
                  'w-full p-4 text-left border rounded-lg transition-colors',
                  selectedMethod === method.id
                    ? 'border-primary-500 bg-primary-50'
                    : 'border-gray-200 hover:border-primary-300 hover:bg-gray-50'
                )}
              >
                <div className="flex items-center gap-3">
                  <method.icon className="w-5 h-5 text-gray-600" />
                  <div>
                    <p className="font-medium text-gray-900">{method.label}</p>
                    <p className="text-sm text-gray-500">{method.description}</p>
                  </div>
                </div>
              </button>
            ))}
          </div>

          {error && (
            <div className="mt-4 p-3 bg-danger-50 rounded-lg flex items-center gap-2 text-sm text-danger-700">
              <AlertCircle className="w-4 h-4" />
              {error}
            </div>
          )}
        </div>
      </Card>
    );
  }

  // Step: Setup (show QR code / secret)
  if (step === 'setup') {
    return (
      <Card className="max-w-md mx-auto">
        <div className="p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-semibold text-gray-900">Setup Authenticator</h2>
            <button onClick={onCancel} className="text-gray-400 hover:text-gray-600">
              <X className="w-5 h-5" />
            </button>
          </div>

          <div className="space-y-6">
            {/* QR Code */}
            <div className="text-center">
              <div className="inline-block p-4 bg-white border rounded-lg">
                {/* In production, render actual QR code */}
                <div className="w-48 h-48 bg-gray-100 flex items-center justify-center">
                  <span className="text-sm text-gray-500">QR Code</span>
                </div>
              </div>
            </div>

            {/* Manual Entry */}
            <div>
              <p className="text-sm text-gray-500 mb-2">
                Or enter this code manually in your authenticator app:
              </p>
              <div className="flex items-center gap-2 p-3 bg-gray-50 rounded-lg">
                <code className="flex-1 font-mono text-sm text-gray-900 break-all">
                  {secret}
                </code>
                <button
                  onClick={() => handleCopy(secret)}
                  className="text-gray-400 hover:text-gray-600"
                >
                  {copiedCode === secret ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                </button>
              </div>
            </div>

            {/* Continue Button */}
            <button
              onClick={() => setStep('verify')}
              className="w-full btn-primary"
            >
              Continue to Verification
            </button>
          </div>
        </div>
      </Card>
    );
  }

  // Step: Verify
  if (step === 'verify') {
    return (
      <Card className="max-w-md mx-auto">
        <div className="p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-semibold text-gray-900">Verify Code</h2>
            <button onClick={onCancel} className="text-gray-400 hover:text-gray-600">
              <X className="w-5 h-5" />
            </button>
          </div>

          <p className="text-sm text-gray-500 mb-6">
            Enter the 6-digit code from your authenticator app.
          </p>

          <div className="space-y-4">
            <div>
              <input
                type="text"
                value={verificationCode}
                onChange={(e) => setVerificationCode(e.target.value.replace(/\D/g, '').slice(0, 6))}
                placeholder="000000"
                className="w-full text-center text-2xl font-mono p-4 border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                maxLength={6}
                autoFocus
              />
            </div>

            {error && (
              <div className="p-3 bg-danger-50 rounded-lg flex items-center gap-2 text-sm text-danger-700">
                <AlertCircle className="w-4 h-4" />
                {error}
              </div>
            )}

            <button
              onClick={handleVerify}
              disabled={verificationCode.length !== 6 || verifyMutation.isPending}
              className="w-full btn-primary"
            >
              {verifyMutation.isPending ? (
                <RefreshCw className="w-4 h-4 animate-spin" />
              ) : (
                'Verify'
              )}
            </button>
          </div>
        </div>
      </Card>
    );
  }

  // Step: Backup Codes
  if (step === 'backup') {
    return (
      <Card className="max-w-md mx-auto">
        <div className="p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-semibold text-gray-900">Backup Codes</h2>
            <button onClick={onCancel} className="text-gray-400 hover:text-gray-600">
              <X className="w-5 h-5" />
            </button>
          </div>

          <div className="p-4 bg-warning-50 border border-warning-200 rounded-lg mb-6">
            <div className="flex items-start gap-3">
              <AlertCircle className="w-5 h-5 text-warning-600 mt-0.5" />
              <div>
                <p className="font-medium text-warning-800">Save your backup codes</p>
                <p className="text-sm text-warning-700 mt-1">
                  Store these codes securely. Each code can only be used once.
                </p>
              </div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-2 mb-6">
            {backupCodes.map((code, i) => (
              <div key={i} className="flex items-center justify-between p-2 bg-gray-50 rounded">
                <code className="font-mono text-sm">{code}</code>
                <button
                  onClick={() => handleCopy(code)}
                  className="text-gray-400 hover:text-gray-600"
                >
                  {copiedCode === code ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
                </button>
              </div>
            ))}
          </div>

          <button
            onClick={() => enableMutation.mutate()}
            disabled={enableMutation.isPending}
            className="w-full btn-primary"
          >
            {enableMutation.isPending ? (
              <RefreshC className="w-4 h-4 animate-spin" />
            ) : (
              'Complete Setup'
            )}
          </button>
        </div>
      </Card>
    );
  }

  // Step: Complete
  return (
    <Card className="max-w-md mx-auto">
      <div className="p-6 text-center">
        <div className="w-16 h-16 bg-success-100 rounded-full flex items-center justify-center mx-auto mb-4">
          <Shield className="w-8 h-8 text-success-600" />
        </div>
        <h2 className="text-xl font-semibold text-gray-900 mb-2">2FA Enabled!</h2>
        <p className="text-sm text-gray-500 mb-6">
          Your account is now protected with two-factor authentication.
        </p>
        <button onClick={onComplete} className="btn-primary">
          Done
        </button>
      </div>
    </Card>
  );
}
