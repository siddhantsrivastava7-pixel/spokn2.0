import React from "react";

interface TextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  variant?: "default" | "compact";
}

export const Textarea: React.FC<TextareaProps> = ({
  className = "",
  variant = "default",
  ...props
}) => {
  const baseClasses =
    "text-[13px] bg-spokn-surface border border-spokn-hairline rounded-lg text-spokn-text text-start transition-all duration-150 placeholder:text-spokn-text-3 hover:bg-spokn-surface-2 hover:border-spokn-hairline-2 focus:outline-none focus:border-spokn-accent-blue/60 focus:ring-2 focus:ring-spokn-accent-blue/25 resize-y";

  const variantClasses = {
    default: "px-3 py-2 min-h-[100px]",
    compact: "px-2 py-1 min-h-[80px]",
  };

  return (
    <textarea
      className={`${baseClasses} ${variantClasses[variant]} ${className}`}
      {...props}
    />
  );
};
