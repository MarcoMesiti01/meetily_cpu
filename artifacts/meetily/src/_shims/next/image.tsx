// Browser stub for `next/image`. Renders a plain <img> with sensible defaults.
import * as React from "react";

export interface ImageProps
  extends Omit<React.ImgHTMLAttributes<HTMLImageElement>, "src" | "alt"> {
  src: string | { src: string; width?: number; height?: number };
  alt: string;
  width?: number | string;
  height?: number | string;
  fill?: boolean;
  priority?: boolean;
  quality?: number;
  loader?: unknown;
  placeholder?: string;
  blurDataURL?: string;
  unoptimized?: boolean;
}

const Image = React.forwardRef<HTMLImageElement, ImageProps>(function Image(
  {
    src,
    alt,
    width,
    height,
    fill,
    priority: _priority,
    quality: _quality,
    loader: _loader,
    placeholder: _placeholder,
    blurDataURL: _blurDataURL,
    unoptimized: _unoptimized,
    style,
    ...rest
  },
  ref,
) {
  const resolvedSrc = typeof src === "string" ? src : src.src;
  const fillStyle: React.CSSProperties | undefined = fill
    ? {
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        objectFit: "cover",
        ...style,
      }
    : style;
  return (
    <img
      ref={ref}
      src={resolvedSrc}
      alt={alt}
      width={fill ? undefined : width}
      height={fill ? undefined : height}
      style={fillStyle}
      {...rest}
    />
  );
});

export default Image;
