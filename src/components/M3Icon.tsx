import React from "react";

interface M3IconProps {
  name: string;
  filled?: boolean;
  size?: number;
  weight?: 300 | 400 | 500 | 600 | 700;
  className?: string;
}

/**
 * Google Material 3 Symbol (Material Symbols Rounded variable font).
 */
export const M3Icon: React.FC<M3IconProps> = ({
  name,
  filled = false,
  size = 18,
  weight = 400,
  className = "",
}) => {
  return (
    <span
      className={`material-symbols-rounded inline-flex items-center justify-center leading-none select-none ${className}`}
      style={{
        fontSize: `${size}px`,
        width: `${size}px`,
        height: `${size}px`,
        fontVariationSettings: `'FILL' ${filled ? 1 : 0}, 'wght' ${weight}, 'GRAD' 0, 'opsz' ${Math.max(20, size)}`,
      }}
      aria-hidden="true"
    >
      {name}
    </span>
  );
};
