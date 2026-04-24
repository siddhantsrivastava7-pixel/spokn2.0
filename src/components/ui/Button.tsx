import React from "react";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?:
    | "primary"
    | "primary-soft"
    | "secondary"
    | "danger"
    | "danger-ghost"
    | "ghost";
  size?: "sm" | "md" | "lg";
}

export const Button: React.FC<ButtonProps> = ({
  children,
  className = "",
  variant = "primary",
  size = "md",
  ...props
}) => {
  const baseClasses =
    "font-medium rounded-lg border focus:outline-none focus-visible:ring-2 focus-visible:ring-spokn-accent-blue/60 focus-visible:ring-offset-0 transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer tracking-tight";

  const variantClasses = {
    primary:
      "text-white bg-spokn-accent-blue border-transparent hover:brightness-110 shadow-spokn-sm",
    "primary-soft":
      "text-spokn-text bg-spokn-accent-blue/15 border-spokn-accent-blue/25 hover:bg-spokn-accent-blue/25",
    secondary:
      "text-spokn-text bg-spokn-surface border-spokn-hairline hover:bg-spokn-surface-2 hover:border-spokn-hairline-2",
    danger:
      "text-white bg-spokn-danger/90 border-transparent hover:bg-spokn-danger",
    "danger-ghost":
      "text-spokn-danger border-transparent hover:bg-spokn-danger/10",
    ghost:
      "text-spokn-text-2 border-transparent hover:text-spokn-text hover:bg-spokn-surface",
  };

  const sizeClasses = {
    sm: "px-2.5 py-1 text-xs",
    md: "px-4 py-1.5 text-[13px]",
    lg: "px-5 py-2 text-sm",
  };

  return (
    <button
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
