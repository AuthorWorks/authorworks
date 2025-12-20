import React from 'react';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  onClick?: () => void;
}

export const Card: React.FC<CardProps> = ({ children, className = '', onClick }) => {
  return (
    <div
      className={`card ${onClick ? 'cursor-pointer hover:border-indigo-500/50' : ''} ${className}`}
      onClick={onClick}
    >
      {children}
    </div>
  );
};
