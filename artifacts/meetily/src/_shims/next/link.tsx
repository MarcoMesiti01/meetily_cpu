// Browser stub for `next/link`. Internal navigation uses wouter's setLocation
// for SPA transitions; external/anchor links fall back to default <a> behavior.
import * as React from "react";
import { useLocation } from "wouter";

export interface LinkProps
  extends Omit<React.AnchorHTMLAttributes<HTMLAnchorElement>, "href"> {
  href: string | { pathname: string; query?: Record<string, string> };
  replace?: boolean;
  scroll?: boolean;
  prefetch?: boolean;
  legacyBehavior?: boolean;
  passHref?: boolean;
  shallow?: boolean;
  locale?: string;
  children?: React.ReactNode;
}

function resolveHref(href: LinkProps["href"]): string {
  if (typeof href === "string") return href;
  const params = href.query
    ? "?" + new URLSearchParams(href.query as Record<string, string>).toString()
    : "";
  return href.pathname + params;
}

const Link = React.forwardRef<HTMLAnchorElement, LinkProps>(function Link(
  {
    href,
    replace,
    scroll: _scroll,
    prefetch: _prefetch,
    legacyBehavior: _legacyBehavior,
    passHref: _passHref,
    shallow: _shallow,
    locale: _locale,
    onClick,
    children,
    ...rest
  },
  ref,
) {
  const [, setLocation] = useLocation();
  const target = resolveHref(href);

  const handleClick = (e: React.MouseEvent<HTMLAnchorElement>) => {
    if (onClick) onClick(e);
    if (e.defaultPrevented) return;
    if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    if (rest.target && rest.target !== "_self") return;
    if (/^[a-z]+:\/\//i.test(target) || target.startsWith("mailto:")) return;
    e.preventDefault();
    setLocation(target, { replace });
  };

  return (
    <a ref={ref} href={target} onClick={handleClick} {...rest}>
      {children}
    </a>
  );
});

export default Link;
