import React, { useEffect } from 'react';
import { X, CheckCircle, AlertCircle, Info } from 'lucide-react';
import { useUIStore } from '../stores/uiStore';

export const ToastContainer: React.FC = () => {
  const { toasts, dismissToast } = useUIStore();

  return (
    <div className="fixed top-4 right-4 z-50 space-y-2">
      {toasts.map((toast) => (
        <Toast key={toast.id} toast={toast} onDismiss={() => dismissToast(toast.id)} />
      ))}
    </div>
  );
};

interface ToastProps {
  toast: { id: string; message: string; type: 'success' | 'error' | 'info' };
  onDismiss: () => void;
}

const Toast: React.FC<ToastProps> = ({ toast, onDismiss }) => {
  useEffect(() => {
    const timer = setTimeout(onDismiss, 5000);
    return () => clearTimeout(timer);
  }, [onDismiss]);

  const icons = {
    success: <CheckCircle className="h-5 w-5 text-green-400" />,
    error: <AlertCircle className="h-5 w-5 text-red-400" />,
    info: <Info className="h-5 w-5 text-indigo-400" />,
  };

  const bgColors = {
    success: 'bg-green-500/20 border-green-500/50',
    error: 'bg-red-500/20 border-red-500/50',
    info: 'bg-indigo-500/20 border-indigo-500/50',
  };

  return (
    <div className={`flex items-start gap-3 min-w-[300px] p-4 rounded-lg border ${bgColors[toast.type]} shadow-xl animate-slide-in`}>
      {icons[toast.type]}
      <p className="flex-1 text-sm text-white">{toast.message}</p>
      <button onClick={onDismiss} className="text-slate-400 hover:text-white">
        <X className="h-4 w-4" />
      </button>
    </div>
  );
};
