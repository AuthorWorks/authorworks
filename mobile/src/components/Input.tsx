import React from 'react';

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export const Input: React.FC<InputProps> = ({ label, error, className = '', ...props }) => {
  return (
    <div className="w-full">
      {label && <label className="block text-sm font-medium text-slate-300 mb-2">{label}</label>}
      <input className={`input ${className}`} {...props} />
      {error && <p className="mt-1 text-sm text-red-400">{error}</p>}
    </div>
  );
};
