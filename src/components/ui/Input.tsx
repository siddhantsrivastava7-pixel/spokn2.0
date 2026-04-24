import React from "react";

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  variant?: "default" | "compact";
}

export const Input: React.FC<InputProps> = ({
  className = "",
  variant = "default",
  disabled,
  ...props
}) => {
  const baseClasses =
    "text-[13px] font-medium bg-spokn-surface border border-spokn-hairline rounded-lg text-spokn-text text-start transition-all duration-150 placeholder:text-spokn-text-3";

  const interactiveClasses = disabled
    ? "opacity-40 cursor-not-allowed"
    : "hover:bg-spokn-surface-2 hover:border-spokn-hairline-2 focus:outline-none focus:border-spokn-accent-blue/60 focus:ring-2 focus:ring-spokn-accent-blue/25";

  const variantClasses = {
    default: "px-3 py-2",
    compact: "px-2 py-1",
  } as const;

  return (
    <input
      className={`${baseClasses} ${variantClasses[variant]} ${interactiveClasses} ${className}`}
      disabled={disabled}
      {...props}
    />
  );
};
